use std::io::{self, Cursor, Write};

use azalea_buf::{AzBuf, AzBufVar, BufReadError};
use azalea_protocol_macros::ClientboundGamePacket;
use azalea_registry::builtin::RecipeBookCategory;

use crate::common::recipe::{Ingredient, RecipeDisplayData, RecipeDisplayId};

#[derive(AzBuf, ClientboundGamePacket, Clone, Debug, PartialEq)]
pub struct ClientboundRecipeBookAdd {
    pub entries: Vec<Entry>,
    pub replace: bool,
}

#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct Entry {
    pub contents: RecipeDisplayEntry,
    pub flags: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecipeDisplayEntry {
    pub id: RecipeDisplayId,
    pub display: RecipeDisplayData,
    pub group: Option<u32>,
    pub category: RecipeBookCategory,
    pub crafting_requirements: Option<Vec<Ingredient>>,
}

impl AzBuf for RecipeDisplayEntry {
    fn azalea_read(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        let id = RecipeDisplayId::azalea_read(buf)?;
        let display = RecipeDisplayData::azalea_read(buf)?;
        let encoded_group = u32::azalea_read_var(buf)?;
        Ok(Self {
            id,
            display,
            group: encoded_group.checked_sub(1),
            category: RecipeBookCategory::azalea_read(buf)?,
            crafting_requirements: Option::<Vec<Ingredient>>::azalea_read(buf)?,
        })
    }

    fn azalea_write(&self, buf: &mut impl Write) -> io::Result<()> {
        self.id.azalea_write(buf)?;
        self.display.azalea_write(buf)?;
        let encoded_group = match self.group {
            Some(group) => group.checked_add(1).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "recipe group ID is too large")
            })?,
            None => 0,
        };
        encoded_group.azalea_write_var(buf)?;
        self.category.azalea_write(buf)?;
        self.crafting_requirements.azalea_write(buf)
    }
}

#[cfg(test)]
mod tests {
    use azalea_registry::builtin::ItemKind;

    use super::*;
    use crate::common::recipe::{ItemSlotDisplay, ShapelessCraftingRecipeDisplay, SlotDisplayData};

    fn entry(group: Option<u32>) -> RecipeDisplayEntry {
        let item = || {
            SlotDisplayData::Item(ItemSlotDisplay {
                item: ItemKind::OakPlanks,
            })
        };
        RecipeDisplayEntry {
            id: RecipeDisplayId(300),
            display: RecipeDisplayData::Shapeless(ShapelessCraftingRecipeDisplay {
                ingredients: vec![item()],
                result: item(),
                crafting_station: item(),
            }),
            group,
            category: RecipeBookCategory::CraftingBuildingBlocks,
            crafting_requirements: None,
        }
    }

    #[test]
    fn recipe_group_uses_optional_varint_encoding() {
        for group in [None, Some(0), Some(127)] {
            let entry = entry(group);
            let mut bytes = Vec::new();
            entry.azalea_write(&mut bytes).unwrap();
            assert_eq!(
                RecipeDisplayEntry::azalea_read(&mut Cursor::new(&bytes)).unwrap(),
                entry
            );
        }
    }
}
