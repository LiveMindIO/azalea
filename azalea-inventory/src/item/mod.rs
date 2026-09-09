use azalea_registry::builtin::ItemKind;

use crate::{
    ItemStack, ItemStackData, components::MaxStackSize, default_components::get_default_component,
};

pub mod consume_effect;

pub trait MaxStackSizeExt {
    /// Get the maximum stack size for this item.
    ///
    /// This is a signed integer to be consistent with the `count` field of
    /// [`ItemStackData`].
    ///
    /// [`ItemStackData`]: crate::ItemStackData
    fn max_stack_size(&self) -> i32;

    /// Whether this item can be stacked with other items.
    ///
    /// This is equivalent to `self.max_stack_size() > 1`.
    fn stackable(&self) -> bool {
        self.max_stack_size() > 1
    }
}

impl MaxStackSizeExt for ItemKind {
    fn max_stack_size(&self) -> i32 {
        get_default_component::<MaxStackSize>(*self).map_or(64, |s| s.count)
    }
}

impl MaxStackSizeExt for ItemStackData {
    fn max_stack_size(&self) -> i32 {
        if self.component_patch.is_removed::<MaxStackSize>() {
            return 1;
        }
        self.get_component::<MaxStackSize>()
            .map_or_else(|| self.kind.max_stack_size(), |size| size.count)
    }
}

impl MaxStackSizeExt for ItemStack {
    fn max_stack_size(&self) -> i32 {
        self.as_present().map_or(0, MaxStackSizeExt::max_stack_size)
    }
}

pub trait CraftingRemainderExt {
    /// The item left behind in a crafting grid cell when this item is
    /// consumed as an ingredient (vanilla's `Item.craftingRemainingItem`).
    fn crafting_remainder(&self) -> Option<ItemKind>;
}

impl CraftingRemainderExt for ItemKind {
    fn crafting_remainder(&self) -> Option<ItemKind> {
        // remainders are per-item registry data, not a data component, so
        // they can't ride `get_default_component`; this is the complete set
        // of `craftRemainder` registrations and must be re-checked against
        // `Items` when the supported version moves
        Some(match self {
            ItemKind::WaterBucket | ItemKind::LavaBucket | ItemKind::MilkBucket => ItemKind::Bucket,
            ItemKind::DragonBreath | ItemKind::HoneyBottle => ItemKind::GlassBottle,
            _ => return None,
        })
    }
}
