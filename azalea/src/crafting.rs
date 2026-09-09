use azalea_client::inventory::{
    ConfirmedContainerClickEvent, GhostRecipe, InventorySyncState, PlaceRecipeEvent, Recipe,
    RecipeBook,
};
use azalea_core::position::BlockPos;
use azalea_entity::inventory::Inventory;
use azalea_inventory::{
    ItemStack, Menu,
    item::MaxStackSizeExt,
    operations::{PickupAllClick, PickupClick},
};
use azalea_registry::builtin::ItemKind;
use bevy_ecs::prelude::Component;
use thiserror::Error;

use crate::{
    Client,
    client_impl::error::{AzaleaResult, MissingComponentError},
    container::{ContainerHandle, ContainerHandleRef},
};

#[derive(Clone, Debug, PartialEq)]
pub struct CraftOutcome {
    pub result: ItemStack,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CraftError {
    #[error(transparent)]
    MissingComponent(#[from] MissingComponentError),
    #[error("the recipe is no longer present in the current recipe book")]
    StaleRecipe,
    #[error("the container is no longer current or initialized")]
    StaleContainer,
    #[error("the current menu is not a crafting menu")]
    WrongMenu,
    #[error("this recipe requires a crafting table")]
    CraftingTableRequired,
    #[error("this recipe display is not supported for crafting")]
    UnsupportedRecipe,
    #[error("the server could not place the recipe with the available ingredients")]
    MissingIngredients,
    #[error("the crafted result does not fit in the player inventory")]
    InventoryFull,
    #[error("the cursor must be empty before crafting")]
    CursorOccupied,
    #[error("a matching result was already present, so recipe placement cannot be verified")]
    PreexistingResult,
    #[error("another crafting operation is already active for this client")]
    Busy,
    #[error("the server rejected or corrected the result click")]
    Rejected,
    #[error("the crafting operation was interrupted")]
    Interrupted,
    #[error("the crafting operation timed out; its final outcome is indeterminate")]
    Timeout,
}

impl Client {
    /// Return the first currently known recipe that produces `result`.
    pub fn recipe_for(&self, result: ItemKind) -> AzaleaResult<Option<Recipe>> {
        self.query_self::<&RecipeBook, _>(|recipe_book| recipe_book.recipes_for(result).next())
    }

    /// Return every currently known recipe that produces `result`.
    pub fn recipes_for(&self, result: ItemKind) -> AzaleaResult<Vec<Recipe>> {
        self.query_self::<&RecipeBook, _>(|recipe_book| recipe_book.recipes_for(result).collect())
    }

    /// Open and verify a crafting-table menu.
    pub async fn open_crafting_table(
        &self,
        pos: BlockPos,
    ) -> AzaleaResult<Option<ContainerHandle>> {
        self.open_crafting_table_with_timeout_ticks(pos, Some(20 * 5))
            .await
    }

    /// Open and verify a crafting-table menu with a configurable timeout.
    pub async fn open_crafting_table_with_timeout_ticks(
        &self,
        pos: BlockPos,
        timeout_ticks: Option<usize>,
    ) -> AzaleaResult<Option<ContainerHandle>> {
        let Some(container) = self
            .open_container_at_with_timeout_ticks(pos, timeout_ticks)
            .await?
        else {
            return Ok(None);
        };
        if matches!(container.menu()?, Some(Menu::Crafting { .. })) {
            Ok(Some(container))
        } else {
            container.close();
            Ok(None)
        }
    }

    /// Ask the server to place one recipe in the supplied crafting menu.
    pub fn place_recipe(
        &self,
        container: &ContainerHandleRef,
        recipe: &Recipe,
        use_max_items: bool,
    ) -> Result<(), CraftError> {
        let expected = recipe.result().ok_or(CraftError::UnsupportedRecipe)?;
        self.craft_snapshot(container.id(), container.generation(), recipe, &expected)?;
        self.ecs.write().trigger(PlaceRecipeEvent {
            entity: self.entity,
            container_id: container.id(),
            menu_generation: container.generation(),
            recipe: recipe.clone(),
            use_max_items,
        });
        Ok(())
    }

    /// Craft one result into the player inventory, timing out after five
    /// seconds.
    pub async fn craft(
        &self,
        container: &ContainerHandleRef,
        recipe: &Recipe,
    ) -> Result<CraftOutcome, CraftError> {
        self.craft_with_timeout_ticks(container, recipe, Some(20 * 5))
            .await
    }

    /// Craft one result and wait for authoritative inventory convergence.
    pub async fn craft_with_timeout_ticks(
        &self,
        container: &ContainerHandleRef,
        recipe: &Recipe,
        timeout_ticks: Option<usize>,
    ) -> Result<CraftOutcome, CraftError> {
        let _guard = self.acquire_craft_guard()?;
        let expected = recipe.result().ok_or(CraftError::UnsupportedRecipe)?;
        let initial =
            self.craft_snapshot(container.id(), container.generation(), recipe, &expected)?;
        if initial.carried.is_present() {
            return Err(CraftError::CursorOccupied);
        }
        if initial.result == expected {
            return Err(CraftError::PreexistingResult);
        }
        let mut ticks = self.get_tick_broadcaster();

        self.place_recipe(container, recipe, false)?;
        self.ecs.write().trigger(ConfirmedContainerClickEvent {
            entity: self.entity,
            window_id: container.id(),
            menu_generation: container.generation(),
            operation: PickupAllClick {
                slot: 0,
                reversed: false,
            }
            .into(),
        });

        let mut elapsed_ticks = 0;
        let placed = loop {
            if ticks.recv().await.is_err() {
                return Err(CraftError::Interrupted);
            }
            elapsed_ticks += 1;
            if timeout_ticks.is_some_and(|timeout| elapsed_ticks > timeout) {
                return Err(CraftError::Timeout);
            }
            let current =
                self.craft_snapshot(container.id(), container.generation(), recipe, &expected)?;
            if current.ghost_revision > initial.ghost_revision
                && current.ghost_recipe.as_ref().is_some_and(|ghost| {
                    ghost.container_id == container.id() && ghost.recipe == *recipe.display()
                })
            {
                return Err(CraftError::MissingIngredients);
            }
            if current.full_content_revision > initial.full_content_revision {
                if current.result == expected {
                    break current;
                }
                return Err(CraftError::Rejected);
            }
            if timeout_ticks.is_some_and(|timeout| elapsed_ticks >= timeout) {
                return Err(CraftError::Timeout);
            }
        };

        if !placed.can_fit_result {
            return Err(CraftError::InventoryFull);
        }

        self.ecs.write().trigger(ConfirmedContainerClickEvent {
            entity: self.entity,
            window_id: container.id(),
            menu_generation: container.generation(),
            operation: PickupClick::Left { slot: Some(0) }.into(),
        });

        let mut current = loop {
            if ticks.recv().await.is_err() {
                return Err(CraftError::Interrupted);
            }
            elapsed_ticks += 1;
            if timeout_ticks.is_some_and(|timeout| elapsed_ticks > timeout) {
                return Err(CraftError::Timeout);
            }
            let current =
                self.craft_snapshot(container.id(), container.generation(), recipe, &expected)?;
            if current.full_content_revision > placed.full_content_revision {
                if current.carried == expected {
                    break current;
                }
                return Err(CraftError::Rejected);
            }
            if timeout_ticks.is_some_and(|timeout| elapsed_ticks >= timeout) {
                return Err(CraftError::Timeout);
            }
        };

        while current.carried.is_present() {
            let Some(destination) = current.destination else {
                return Err(CraftError::InventoryFull);
            };
            self.ecs.write().trigger(ConfirmedContainerClickEvent {
                entity: self.entity,
                window_id: container.id(),
                menu_generation: container.generation(),
                operation: PickupClick::Left {
                    slot: Some(destination as u16),
                }
                .into(),
            });
            let previous = current;
            loop {
                if ticks.recv().await.is_err() {
                    return Err(CraftError::Interrupted);
                }
                elapsed_ticks += 1;
                if timeout_ticks.is_some_and(|timeout| elapsed_ticks > timeout) {
                    return Err(CraftError::Timeout);
                }
                current =
                    self.craft_snapshot(container.id(), container.generation(), recipe, &expected)?;
                if current.full_content_revision > previous.full_content_revision {
                    if current.carried.count() < previous.carried.count() {
                        break;
                    }
                    return Err(CraftError::Rejected);
                }
                if timeout_ticks.is_some_and(|timeout| elapsed_ticks >= timeout) {
                    return Err(CraftError::Timeout);
                }
            }
        }

        Ok(CraftOutcome { result: expected })
    }

    fn craft_snapshot(
        &self,
        container_id: i32,
        menu_generation: u64,
        recipe: &Recipe,
        expected: &ItemStack,
    ) -> Result<CraftSnapshot, CraftError> {
        self.query_self::<(&Inventory, &InventorySyncState, &RecipeBook), _>(
            |(inventory, sync_state, recipe_book)| {
                if inventory.id != container_id {
                    return Err(CraftError::Interrupted);
                }
                if sync_state.menu_generation() != menu_generation {
                    return Err(CraftError::StaleContainer);
                }
                if !sync_state.is_initialized(container_id) {
                    return Err(CraftError::StaleContainer);
                }
                if !recipe_book.contains(recipe) {
                    return Err(CraftError::StaleRecipe);
                }

                match inventory.menu() {
                    Menu::Player(_) if !recipe.fits_crafting_grid(2, 2) => {
                        return Err(CraftError::CraftingTableRequired);
                    }
                    Menu::Player(_) => {}
                    Menu::Crafting { .. } if !recipe.fits_crafting_grid(3, 3) => {
                        return Err(CraftError::UnsupportedRecipe);
                    }
                    Menu::Crafting { .. } => {}
                    _ => return Err(CraftError::WrongMenu),
                }

                let player_slots = &inventory.menu().slots()[inventory.menu().player_slots_range()];
                let can_fit_result = can_fit(player_slots, expected);

                Ok(CraftSnapshot {
                    result: inventory
                        .menu()
                        .slot(0)
                        .cloned()
                        .unwrap_or(ItemStack::Empty),
                    carried: inventory.carried.clone(),
                    destination: destination_slot(inventory.menu(), &inventory.carried),
                    can_fit_result,
                    full_content_revision: sync_state.full_content_revision(),
                    ghost_revision: recipe_book.ghost_revision(),
                    ghost_recipe: recipe_book.ghost_recipe().cloned(),
                })
            },
        )?
    }

    fn acquire_craft_guard(&self) -> Result<CraftGuard, CraftError> {
        let mut ecs = self.ecs.write();
        if ecs
            .get::<CraftOperationState>(self.entity)
            .is_some_and(|state| state.active)
        {
            return Err(CraftError::Busy);
        }
        ecs.entity_mut(self.entity)
            .insert(CraftOperationState { active: true });
        Ok(CraftGuard {
            client: self.clone(),
        })
    }
}

#[derive(Component)]
struct CraftOperationState {
    active: bool,
}

struct CraftGuard {
    client: Client,
}

impl Drop for CraftGuard {
    fn drop(&mut self) {
        if let Some(mut state) = self
            .client
            .ecs
            .write()
            .get_mut::<CraftOperationState>(self.client.entity)
        {
            state.active = false;
        }
    }
}

struct CraftSnapshot {
    result: ItemStack,
    carried: ItemStack,
    destination: Option<usize>,
    can_fit_result: bool,
    full_content_revision: u64,
    ghost_revision: u64,
    ghost_recipe: Option<GhostRecipe>,
}

fn can_fit(slots: &[ItemStack], expected: &ItemStack) -> bool {
    expected.as_present().is_some_and(|expected| {
        let capacity: i32 = slots
            .iter()
            .map(|slot| match slot {
                ItemStack::Empty => expected.max_stack_size(),
                ItemStack::Present(slot) if slot.is_same_item_and_components(expected) => {
                    expected.max_stack_size() - slot.count
                }
                _ => 0,
            })
            .sum();
        capacity >= expected.count
    })
}

fn destination_slot(menu: &Menu, carried: &ItemStack) -> Option<usize> {
    let carried = carried.as_present()?;
    let mut range = menu.player_slots_range();
    range
        .clone()
        .find(|&index| {
            menu.slot(index).is_some_and(|slot| {
                slot.as_present().is_some_and(|slot| {
                    slot.is_same_item_and_components(carried)
                        && slot.count < carried.max_stack_size()
                })
            })
        })
        .or_else(|| range.find(|&index| menu.slot(index).is_some_and(ItemStack::is_empty)))
}

#[cfg(test)]
mod tests {
    use azalea_inventory::components::MaxStackSize;

    use super::*;

    #[test]
    fn result_capacity_can_span_multiple_stacks() {
        let slots = [
            ItemStack::new(ItemKind::OakPlanks, 62),
            ItemStack::new(ItemKind::OakPlanks, 62),
        ];
        assert!(can_fit(&slots, &ItemStack::new(ItemKind::OakPlanks, 4)));
    }

    #[test]
    fn result_capacity_honors_stack_max_size_component() {
        let result =
            ItemStack::new(ItemKind::OakPlanks, 4).with_component(MaxStackSize { count: 2 });
        assert!(!can_fit(&[ItemStack::Empty], &result));
        assert!(can_fit(&[ItemStack::Empty, ItemStack::Empty], &result));
    }
}
