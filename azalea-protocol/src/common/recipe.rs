use std::io::{self, Cursor, Write};

use azalea_buf::{AzBuf, AzBufVar, BufReadError};
use azalea_inventory::ItemStack;
use azalea_registry::{
    HolderSet,
    builtin::{DataComponentKind, ItemKind},
    data::TrimPattern,
    identifier::Identifier,
};

/// A server-assigned recipe display identifier.
///
/// These identifiers are only valid for the current recipe-book generation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RecipeDisplayId(pub u32);

impl AzBuf for RecipeDisplayId {
    fn azalea_read(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        Ok(Self(u32::azalea_read_var(buf)?))
    }

    fn azalea_write(&self, buf: &mut impl Write) -> io::Result<()> {
        self.0.azalea_write_var(buf)
    }
}

/// [`azalea_registry::builtin::RecipeDisplay`]
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub enum RecipeDisplayData {
    Shapeless(ShapelessCraftingRecipeDisplay),
    Shaped(ShapedCraftingRecipeDisplay),
    Furnace(FurnaceRecipeDisplay),
    Stonecutter(StonecutterRecipeDisplay),
    Smithing(SmithingRecipeDisplay),
}

#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct ShapelessCraftingRecipeDisplay {
    pub ingredients: Vec<SlotDisplayData>,
    pub result: SlotDisplayData,
    pub crafting_station: SlotDisplayData,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct ShapedCraftingRecipeDisplay {
    #[var]
    pub width: u32,
    #[var]
    pub height: u32,
    pub ingredients: Vec<SlotDisplayData>,
    pub result: SlotDisplayData,
    pub crafting_station: SlotDisplayData,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct FurnaceRecipeDisplay {
    pub ingredient: SlotDisplayData,
    pub fuel: SlotDisplayData,
    pub result: SlotDisplayData,
    pub crafting_station: SlotDisplayData,
    #[var]
    pub duration: u32,
    pub experience: f32,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct StonecutterRecipeDisplay {
    pub input: SlotDisplayData,
    pub result: SlotDisplayData,
    pub crafting_station: SlotDisplayData,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct SmithingRecipeDisplay {
    pub template: SlotDisplayData,
    pub base: SlotDisplayData,
    pub addition: SlotDisplayData,
    pub result: SlotDisplayData,
    pub crafting_station: SlotDisplayData,
}

#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct Ingredient {
    pub allowed: HolderSet<ItemKind, Identifier>,
}

/// [`azalea_registry::builtin::SlotDisplay`]
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub enum SlotDisplayData {
    Empty,
    AnyFuel,
    WithAnyPotion(Box<WithAnyPotionSlotDisplay>),
    OnlyWithComponent(Box<OnlyWithComponentSlotDisplay>),
    Item(ItemSlotDisplay),
    ItemStack(ItemStackSlotDisplay),
    Tag(TagSlotDisplay),
    Dyed(Box<DyedSlotDemo>),
    SmithingTrim(Box<SmithingTrimDemoSlotDisplay>),
    WithRemainder(Box<WithRemainderSlotDisplay>),
    Composite(CompositeSlotDisplay),
}

#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct WithAnyPotionSlotDisplay {
    pub contents: SlotDisplayData,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct OnlyWithComponentSlotDisplay {
    pub contents: SlotDisplayData,
    pub component: DataComponentKind,
}

#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct ItemSlotDisplay {
    pub item: ItemKind,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ItemStackSlotDisplay {
    pub stack: ItemStack,
}

// vanilla's SlotDisplay.ItemStackSlotDisplay carries the stack in the
// template wire form (item id, count, patch), not the count-first codec
// inventory slots use
impl AzBuf for ItemStackSlotDisplay {
    fn azalea_read(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        Ok(ItemStackSlotDisplay {
            stack: ItemStack::azalea_read_template(buf)?,
        })
    }
    fn azalea_write(&self, buf: &mut impl Write) -> io::Result<()> {
        self.stack.azalea_write_template(buf)
    }
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct DyedSlotDemo {
    pub dye: SlotDisplayData,
    pub target: SlotDisplayData,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct TagSlotDisplay {
    pub tag: Identifier,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct SmithingTrimDemoSlotDisplay {
    pub base: SlotDisplayData,
    pub material: SlotDisplayData,
    pub pattern: TrimPattern,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct WithRemainderSlotDisplay {
    pub input: SlotDisplayData,
    pub remainder: SlotDisplayData,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct CompositeSlotDisplay {
    pub contents: Vec<SlotDisplayData>,
}
