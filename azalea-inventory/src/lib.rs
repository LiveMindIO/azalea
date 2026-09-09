#![doc = include_str!("../README.md")]
#![feature(min_specialization)]

pub mod components;
pub mod default_components;
pub mod item;
pub mod operations;
mod slot;

use std::ops::{Deref, DerefMut, RangeInclusive};

use azalea_inventory_macros::declare_menus;
pub use slot::{DataComponentPatch, ItemStack, ItemStackData};

// TODO: remove this here and in azalea-inventory-macros when rust makes
// Default be implemented for all array sizes
// https://github.com/rust-lang/rust/issues/61415

/// A fixed-size list of [`ItemStack`]s.
#[derive(Clone, Debug)]
pub struct SlotList<const N: usize>([ItemStack; N]);
impl<const N: usize> Deref for SlotList<N> {
    type Target = [ItemStack; N];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<const N: usize> DerefMut for SlotList<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<const N: usize> Default for SlotList<N> {
    fn default() -> Self {
        SlotList([(); N].map(|_| ItemStack::Empty))
    }
}
impl<const N: usize> SlotList<N> {
    pub fn new(items: [ItemStack; N]) -> Self {
        SlotList(items)
    }
}

impl Menu {
    /// Get the [`Player`] from this [`Menu`].
    ///
    /// # Panics
    ///
    /// Will panic if the menu isn't `Menu::Player`.
    pub fn as_player(&self) -> &Player {
        self.try_as_player()
            .expect("Called `Menu::as_player` on a menu that wasn't `Player`.")
    }
    /// Get the [`Player`] from this [`Menu`], or returns `None` if the menu
    /// isn't a player menu.
    pub fn try_as_player(&self) -> Option<&Player> {
        if let Menu::Player(player) = &self {
            Some(player)
        } else {
            None
        }
    }

    /// Same as [`Menu::as_player`], but returns a mutable reference to the
    /// [`Player`].
    ///
    /// # Panics
    ///
    /// Will panic if the menu isn't `Menu::Player`.
    pub fn as_player_mut(&mut self) -> &mut Player {
        self.try_as_player_mut()
            .expect("Called `Menu::as_player_mut` on a menu that wasn't `Player`.")
    }
    /// Same as [`Menu::try_as_player`], but returns a mutable reference to the
    /// [`Player`].
    pub fn try_as_player_mut(&mut self) -> Option<&mut Player> {
        if let Menu::Player(player) = self {
            Some(player)
        } else {
            None
        }
    }
}

// the player inventory part is always the last 36 slots (except in the Player
// menu), so we don't have to explicitly specify it

// Client {
//     ...
//     pub menu: Menu,
//     pub inventory: Arc<[Slot; 36]>
// }

// Generate a `struct Player`, `enum Menu`, and `impl Menu`.
// a "player" field gets implicitly added with the player inventory

declare_menus! {
    Player {
        craft_result: 1,
        craft: 4,
        armor: 4,
        inventory: 36,
        offhand: 1,
    },
    Generic9x1 {
        contents: 9,
    },
    Generic9x2 {
        contents: 18,
    },
    Generic9x3 {
        contents: 27,
    },
    Generic9x4 {
        contents: 36,
    },
    Generic9x5 {
        contents: 45,
    },
    Generic9x6 {
        contents: 54,
    },
    Generic3x3 {
        contents: 9,
    },
    Crafter3x3 {
        contents: 9,
    },
    Anvil {
        first: 1,
        second: 1,
        result: 1,
    },
    Beacon {
        payment: 1,
    },
    BlastFurnace {
        ingredient: 1,
        fuel: 1,
        result: 1,
    },
    BrewingStand {
        bottles: 3,
        ingredient: 1,
        fuel: 1,
    },
    Crafting {
        result: 1,
        grid: 9,
    },
    Enchantment {
        item: 1,
        lapis: 1,
    },
    Furnace {
        ingredient: 1,
        fuel: 1,
        result: 1,
    },
    Grindstone {
        input: 1,
        additional: 1,
        result: 1,
    },
    Hopper {
        contents: 5,
    },
    Lectern {
        book: 1,
    },
    Loom {
        banner: 1,
        dye: 1,
        pattern: 1,
        result: 1,
    },
    Merchant {
        payments: 2,
        result: 1,
    },
    ShulkerBox {
        contents: 27,
    },
    Smithing {
        template: 1,
        base: 1,
        additional: 1,
        result: 1,
    },
    Smoker {
        ingredient: 1,
        fuel: 1,
        result: 1,
    },
    CartographyTable {
        map: 1,
        additional: 1,
        result: 1,
    },
    Stonecutter {
        input: 1,
        result: 1,
    },
}

#[cfg(test)]
mod tests {
    use azalea_registry::builtin::ItemKind;

    use super::*;
    use crate::components::MaxStackSize;

    /// Moving out of a container walks the player slots in reverse, so with
    /// nothing to merge into, the stack lands in the last hotbar slot — the
    /// slot vanilla picks. Anything else desyncs the simulation until the
    /// server corrects it.
    #[test]
    fn quick_move_out_of_a_container_fills_the_hotbar_first() {
        let mut menu = Menu::Generic9x3 {
            contents: Default::default(),
            player: Default::default(),
        };
        let dirt = ItemStack::new(ItemKind::Dirt, 20);
        *menu.slot_mut(3).unwrap() = dirt.clone();

        let moved = menu.quick_move_stack(3);
        assert!(moved.is_present());
        assert_eq!(menu.slot(3).unwrap(), &ItemStack::Empty);

        let last_player_slot = *menu.player_slots_range().end();
        for (i, slot) in menu.slots().iter().enumerate() {
            if i == last_player_slot {
                assert_eq!(slot, &dirt);
            } else {
                assert_eq!(slot, &ItemStack::Empty, "slot {i} shouldn't be touched");
            }
        }
    }

    /// Merging tops the target stack up to its limit and leaves the
    /// remainder to continue down the line, following vanilla's arithmetic;
    /// overwriting the target with the source count deletes items from the
    /// model.
    #[test]
    fn quick_move_merges_with_vanilla_arithmetic() {
        let dirt = |count| ItemStack::new(ItemKind::Dirt, count);

        // simple merge: 30 in the inventory + 20 from the chest = 50
        let mut menu = Menu::Generic9x3 {
            contents: Default::default(),
            player: Default::default(),
        };
        *menu.slot_mut(3).unwrap() = dirt(20);
        *menu.slot_mut(40).unwrap() = dirt(30);
        menu.quick_move_stack(3);
        assert_eq!(menu.slot(3).unwrap(), &ItemStack::Empty);
        assert_eq!(menu.slot(40).unwrap(), &dirt(50));

        // overflow: the target caps at 64 and the remainder goes to the
        // last empty slot (the reverse scan again)
        let mut menu = Menu::Generic9x3 {
            contents: Default::default(),
            player: Default::default(),
        };
        *menu.slot_mut(3).unwrap() = dirt(20);
        *menu.slot_mut(40).unwrap() = dirt(60);
        menu.quick_move_stack(3);
        assert_eq!(menu.slot(3).unwrap(), &ItemStack::Empty);
        assert_eq!(menu.slot(40).unwrap(), &dirt(64));
        assert_eq!(
            menu.slot(*menu.player_slots_range().end()).unwrap(),
            &dirt(16)
        );
    }

    #[test]
    fn quick_move_honors_stack_max_size_component() {
        let dirt = |count| {
            ItemStack::new(ItemKind::Dirt, count).with_component(MaxStackSize { count: 16 })
        };
        let mut menu = Menu::Generic9x3 {
            contents: Default::default(),
            player: Default::default(),
        };
        *menu.slot_mut(3).unwrap() = dirt(10);
        *menu.slot_mut(40).unwrap() = dirt(12);

        menu.quick_move_stack(3);

        assert_eq!(menu.slot(3).unwrap(), &ItemStack::Empty);
        assert_eq!(menu.slot(40).unwrap(), &dirt(16));
        assert_eq!(
            menu.slot(*menu.player_slots_range().end()).unwrap(),
            &dirt(6)
        );
    }

    #[test]
    fn quick_move_allows_component_stack_sizes_above_64() {
        let dirt = |count| {
            ItemStack::new(ItemKind::Dirt, count).with_component(MaxStackSize { count: 99 })
        };
        let mut menu = Menu::Generic9x3 {
            contents: Default::default(),
            player: Default::default(),
        };
        *menu.slot_mut(3).unwrap() = dirt(35);
        *menu.slot_mut(40).unwrap() = dirt(64);

        menu.quick_move_stack(3);

        assert_eq!(menu.slot(3).unwrap(), &ItemStack::Empty);
        assert_eq!(menu.slot(40).unwrap(), &dirt(99));
    }

    #[test]
    fn quick_move_splits_stacked_bottles_into_brewing_slots() {
        let mut menu = Menu::BrewingStand {
            bottles: Default::default(),
            ingredient: ItemStack::Empty,
            fuel: ItemStack::Empty,
            player: Default::default(),
        };
        let source = *menu.player_slots_range().start();
        *menu.slot_mut(source).unwrap() = ItemStack::new(ItemKind::GlassBottle, 3);

        menu.quick_move_stack(source);

        assert_eq!(
            menu.slot(*Menu::BREWING_STAND_BOTTLES_SLOTS.start())
                .unwrap(),
            &ItemStack::new(ItemKind::GlassBottle, 1)
        );
        assert_eq!(
            menu.slot(source).unwrap(),
            &ItemStack::new(ItemKind::GlassBottle, 2)
        );
    }

    #[test]
    fn quick_move_does_not_fall_back_when_brewing_fuel_is_full() {
        let mut menu = Menu::BrewingStand {
            bottles: Default::default(),
            ingredient: ItemStack::Empty,
            fuel: ItemStack::new(ItemKind::BlazePowder, 60),
            player: Default::default(),
        };
        let source = *menu.player_slots_range().start();
        *menu.slot_mut(source).unwrap() = ItemStack::new(ItemKind::BlazePowder, 10);

        menu.quick_move_stack(source);
        assert!(menu.quick_move_stack(source).is_empty());

        assert_eq!(
            menu.slot(Menu::BREWING_STAND_FUEL_SLOT).unwrap(),
            &ItemStack::new(ItemKind::BlazePowder, 64)
        );
        assert_eq!(
            menu.slot(source).unwrap(),
            &ItemStack::new(ItemKind::BlazePowder, 6)
        );
        assert_eq!(
            menu.slot(Menu::BREWING_STAND_INGREDIENT_SLOT).unwrap(),
            &ItemStack::Empty
        );
    }

    #[test]
    fn bundles_can_quick_move_into_shulker_boxes() {
        let mut menu = Menu::ShulkerBox {
            contents: Default::default(),
            player: Default::default(),
        };
        let source = *menu.player_slots_range().start();
        *menu.slot_mut(source).unwrap() = ItemStack::new(ItemKind::Bundle, 1);

        menu.quick_move_stack(source);

        assert_eq!(menu.slot(source).unwrap(), &ItemStack::Empty);
        assert_eq!(
            menu.slot(*Menu::SHULKER_BOX_CONTENTS_SLOTS.start())
                .unwrap(),
            &ItemStack::new(ItemKind::Bundle, 1)
        );
    }

    /// Vanilla has no inventory/hotbar fallback in plain containers: when
    /// the chest is full the shift-click does nothing at all.
    #[test]
    fn quick_move_into_a_full_container_moves_nothing() {
        let mut menu = Menu::Generic9x3 {
            contents: Default::default(),
            player: Default::default(),
        };
        let stone = ItemStack::new(ItemKind::Stone, 64);
        for i in Menu::GENERIC9X3_CONTENTS_SLOTS {
            *menu.slot_mut(i).unwrap() = stone.clone();
        }
        let dirt = ItemStack::new(ItemKind::Dirt, 20);
        let source = *menu.player_slots_range().start();
        *menu.slot_mut(source).unwrap() = dirt.clone();

        let moved = menu.quick_move_stack(source);
        assert!(moved.is_empty());
        assert_eq!(menu.slot(source).unwrap(), &dirt);
    }

    /// Gear shift-clicked in the player's own inventory auto-equips into
    /// its empty equipment slot (the `equippable` component decides which),
    /// like vanilla InventoryMenu.
    #[test]
    fn quick_move_in_player_menu_auto_equips_armor() {
        let mut menu = Menu::Player(Player::default());
        let helmet = ItemStack::new(ItemKind::DiamondHelmet, 1);
        let boots = ItemStack::new(ItemKind::DiamondBoots, 1);
        *menu.slot_mut(10).unwrap() = helmet.clone();
        *menu.slot_mut(11).unwrap() = boots.clone();

        menu.quick_move_stack(10);
        menu.quick_move_stack(11);
        assert_eq!(menu.slot(5).unwrap(), &helmet, "helmet slot");
        assert_eq!(menu.slot(8).unwrap(), &boots, "boots slot");

        let pumpkin = ItemStack::new(ItemKind::CarvedPumpkin, 3);
        *menu.slot_mut(12).unwrap() = pumpkin.clone();
        *menu.slot_mut(5).unwrap() = ItemStack::Empty;
        menu.quick_move_stack(12);
        assert_eq!(
            menu.slot(5).unwrap(),
            &ItemStack::new(ItemKind::CarvedPumpkin, 1),
            "armor slots only accept one stackable equippable item"
        );
        assert_eq!(
            menu.slot(12).unwrap(),
            &ItemStack::new(ItemKind::CarvedPumpkin, 2)
        );

        // with the helmet slot occupied, a second helmet toggles to the
        // hotbar instead
        let second = ItemStack::new(ItemKind::IronHelmet, 1);
        *menu.slot_mut(10).unwrap() = second.clone();
        menu.quick_move_stack(10);
        assert_eq!(menu.slot(*Player::HOTBAR_SLOTS.start()).unwrap(), &second);
    }
}
