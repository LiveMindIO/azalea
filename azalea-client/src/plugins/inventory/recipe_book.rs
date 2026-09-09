use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicU64, Ordering},
};

use azalea_inventory::{ItemStack, ItemStackData};
use azalea_protocol::{
    common::recipe::{Ingredient, RecipeDisplayData, RecipeDisplayId, SlotDisplayData},
    packets::game::{
        c_recipe_book_add::RecipeDisplayEntry,
        c_recipe_book_settings::RecipeBookSettings,
        c_update_recipes::{RecipePropertySet, SingleInputEntry},
    },
};
use azalea_registry::{
    builtin::{ItemKind, RecipeBookCategory},
    identifier::Identifier,
};
use bevy_ecs::prelude::Component;
use indexmap::IndexMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Recipe {
    generation: u64,
    entry: RecipeDisplayEntry,
}

impl Recipe {
    pub fn id(&self) -> RecipeDisplayId {
        self.entry.id
    }

    pub fn display(&self) -> &RecipeDisplayData {
        &self.entry.display
    }

    pub fn group(&self) -> Option<u32> {
        self.entry.group
    }

    pub fn category(&self) -> RecipeBookCategory {
        self.entry.category
    }

    pub fn crafting_requirements(&self) -> Option<&[Ingredient]> {
        self.entry.crafting_requirements.as_deref()
    }

    pub fn result_item(&self) -> Option<ItemKind> {
        self.result().map(|stack| stack.kind())
    }

    pub fn result(&self) -> Option<ItemStack> {
        recipe_result(self.display()).and_then(slot_display_stack)
    }

    pub fn fits_crafting_grid(&self, width: u32, height: u32) -> bool {
        match self.display() {
            RecipeDisplayData::Shaped(recipe) => recipe.width <= width && recipe.height <= height,
            RecipeDisplayData::Shapeless(recipe) => {
                recipe.ingredients.len() <= (width * height) as usize
            }
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GhostRecipe {
    pub container_id: i32,
    pub recipe: RecipeDisplayData,
}

#[derive(Component, Clone, Debug)]
pub struct RecipeBook {
    item_sets: HashMap<Identifier, RecipePropertySet>,
    stonecutter_recipes: Vec<SingleInputEntry>,
    recipes: IndexMap<RecipeDisplayId, RecipeDisplayEntry>,
    highlighted: HashSet<RecipeDisplayId>,
    settings: RecipeBookSettings,
    generation: u64,
    revision: u64,
    ghost_revision: u64,
    ghost_recipe: Option<GhostRecipe>,
}

impl RecipeBook {
    pub fn recipes(&self) -> impl Iterator<Item = Recipe> + '_ {
        self.recipes.values().cloned().map(|entry| Recipe {
            generation: self.generation,
            entry,
        })
    }

    pub fn recipes_for(&self, result: ItemKind) -> impl Iterator<Item = Recipe> + '_ {
        self.recipes()
            .filter(move |recipe| recipe.result_item() == Some(result))
    }

    pub fn recipe(&self, id: RecipeDisplayId) -> Option<Recipe> {
        self.recipes.get(&id).cloned().map(|entry| Recipe {
            generation: self.generation,
            entry,
        })
    }

    pub fn contains(&self, recipe: &Recipe) -> bool {
        recipe.generation == self.generation
            && self.recipes.get(&recipe.id()) == Some(&recipe.entry)
    }

    pub fn is_highlighted(&self, id: RecipeDisplayId) -> bool {
        self.highlighted.contains(&id)
    }

    pub fn settings(&self) -> &RecipeBookSettings {
        &self.settings
    }

    pub fn item_sets(&self) -> &HashMap<Identifier, RecipePropertySet> {
        &self.item_sets
    }

    pub fn stonecutter_recipes(&self) -> &[SingleInputEntry] {
        &self.stonecutter_recipes
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn ghost_recipe(&self) -> Option<&GhostRecipe> {
        self.ghost_recipe.as_ref()
    }

    pub fn ghost_revision(&self) -> u64 {
        self.ghost_revision
    }

    pub(crate) fn replace_recipe_data(
        &mut self,
        item_sets: HashMap<Identifier, RecipePropertySet>,
        stonecutter_recipes: Vec<SingleInputEntry>,
    ) {
        self.item_sets = item_sets;
        self.stonecutter_recipes = stonecutter_recipes;
        self.recipes.clear();
        self.highlighted.clear();
        self.generation = self.generation.wrapping_add(1);
        self.revision = self.revision.wrapping_add(1);
    }

    pub(crate) fn add(
        &mut self,
        entries: impl IntoIterator<Item = (RecipeDisplayEntry, u8)>,
        replace: bool,
    ) {
        if replace {
            self.recipes.clear();
            self.highlighted.clear();
            self.generation = self.generation.wrapping_add(1);
        }
        for (entry, flags) in entries {
            if flags & 2 != 0 {
                self.highlighted.insert(entry.id);
            } else {
                self.highlighted.remove(&entry.id);
            }
            self.recipes.insert(entry.id, entry);
        }
        self.revision = self.revision.wrapping_add(1);
    }

    pub(crate) fn remove(&mut self, recipes: &[RecipeDisplayId]) {
        for recipe in recipes {
            self.recipes.shift_remove(recipe);
            self.highlighted.remove(recipe);
        }
        self.revision = self.revision.wrapping_add(1);
    }

    pub(crate) fn set_settings(&mut self, settings: RecipeBookSettings) {
        self.settings = settings;
        self.revision = self.revision.wrapping_add(1);
    }

    pub(crate) fn set_ghost_recipe(&mut self, ghost_recipe: GhostRecipe) {
        self.ghost_recipe = Some(ghost_recipe);
        self.ghost_revision = self.ghost_revision.wrapping_add(1);
        self.revision = self.revision.wrapping_add(1);
    }
}

impl Default for RecipeBook {
    fn default() -> Self {
        static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);
        Self {
            item_sets: HashMap::new(),
            stonecutter_recipes: Vec::new(),
            recipes: IndexMap::new(),
            highlighted: HashSet::new(),
            settings: RecipeBookSettings::default(),
            generation: NEXT_GENERATION.fetch_add(1, Ordering::Relaxed),
            revision: 0,
            ghost_revision: 0,
            ghost_recipe: None,
        }
    }
}

fn recipe_result(recipe: &RecipeDisplayData) -> Option<&SlotDisplayData> {
    match recipe {
        RecipeDisplayData::Shapeless(recipe) => Some(&recipe.result),
        RecipeDisplayData::Shaped(recipe) => Some(&recipe.result),
        RecipeDisplayData::Furnace(recipe) => Some(&recipe.result),
        RecipeDisplayData::Stonecutter(recipe) => Some(&recipe.result),
        RecipeDisplayData::Smithing(recipe) => Some(&recipe.result),
    }
}

fn slot_display_stack(display: &SlotDisplayData) -> Option<ItemStack> {
    match display {
        SlotDisplayData::Item(display) => {
            Some(ItemStack::Present(ItemStackData::new(display.item, 1)))
        }
        SlotDisplayData::ItemStack(display) if display.stack.is_present() => {
            Some(display.stack.clone())
        }
        SlotDisplayData::Composite(display) => display.contents.iter().find_map(slot_display_stack),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_generations_do_not_revalidate_old_recipes() {
        let first = RecipeBook::default();
        let second = RecipeBook::default();
        assert_ne!(first.generation, second.generation);
    }
}
