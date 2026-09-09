use std::ops::RangeInclusive;

use azalea_buf::AzBuf;
use azalea_registry::builtin::ItemKind;

use crate::{
    AnvilMenuLocation, BeaconMenuLocation, BlastFurnaceMenuLocation, BrewingStandMenuLocation,
    CartographyTableMenuLocation, Crafter3x3MenuLocation, CraftingMenuLocation,
    EnchantmentMenuLocation, FurnaceMenuLocation, Generic3x3MenuLocation, Generic9x1MenuLocation,
    Generic9x2MenuLocation, Generic9x3MenuLocation, Generic9x4MenuLocation, Generic9x5MenuLocation,
    Generic9x6MenuLocation, GrindstoneMenuLocation, HopperMenuLocation, ItemStack, ItemStackData,
    LecternMenuLocation, LoomMenuLocation, Menu, MenuLocation, MerchantMenuLocation, Player,
    PlayerMenuLocation, ShulkerBoxMenuLocation, SmithingMenuLocation, SmokerMenuLocation,
    StonecutterMenuLocation,
    components::{self, EquipmentSlot},
    item::{CraftingRemainderExt, MaxStackSizeExt},
};

/// A type of click in a Minecraft inventory.
#[derive(Clone, Debug)]
pub enum ClickOperation {
    Pickup(PickupClick),
    QuickMove(QuickMoveClick),
    Swap(SwapClick),
    Clone(CloneClick),
    Throw(ThrowClick),
    QuickCraft(QuickCraftClick),
    PickupAll(PickupAllClick),
}

#[derive(Clone, Debug)]
pub enum PickupClick {
    /// Left mouse click.
    ///
    /// Note that in the protocol, None is represented as -999.
    Left { slot: Option<u16> },
    /// Right mouse click.
    ///
    /// Note that in the protocol, None is represented as -999.
    Right { slot: Option<u16> },
    /// Drop cursor stack.
    LeftOutside,
    /// Drop cursor single item.
    RightOutside,
}
impl From<PickupClick> for ClickOperation {
    fn from(click: PickupClick) -> Self {
        ClickOperation::Pickup(click)
    }
}

/// Shift click
#[derive(Clone, Debug)]
pub enum QuickMoveClick {
    /// Shift + left mouse click
    Left { slot: u16 },
    /// Shift + right mouse click (identical behavior)
    Right { slot: u16 },
}
impl From<QuickMoveClick> for ClickOperation {
    fn from(click: QuickMoveClick) -> Self {
        ClickOperation::QuickMove(click)
    }
}

/// Used when you press number keys or F in an inventory.
#[derive(Clone, Debug)]
pub struct SwapClick {
    pub source_slot: u16,
    /// 0-8 for hotbar slots, 40 for offhand, everything else is treated as a
    /// slot index.
    pub target_slot: u8,
}

impl From<SwapClick> for ClickOperation {
    fn from(click: SwapClick) -> Self {
        ClickOperation::Swap(click)
    }
}
/// Middle click, only defined for creative players in non-player
/// inventories.
#[derive(Clone, Debug)]
pub struct CloneClick {
    pub slot: u16,
}
impl From<CloneClick> for ClickOperation {
    fn from(click: CloneClick) -> Self {
        ClickOperation::Clone(click)
    }
}
#[derive(Clone, Debug)]
pub enum ThrowClick {
    /// Drop key (Q)
    Single { slot: u16 },
    /// Ctrl + drop key (Q)
    All { slot: u16 },
}
impl From<ThrowClick> for ClickOperation {
    fn from(click: ThrowClick) -> Self {
        ClickOperation::Throw(click)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuickCraftClick {
    pub kind: QuickCraftKind,
    pub status: QuickCraftStatus,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuickCraftKind {
    Left,
    Right,
    Middle,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuickCraftStatusKind {
    /// Starting drag
    Start,
    /// Add slot
    Add,
    /// Ending drag
    End,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuickCraftStatus {
    /// Starting drag
    Start,
    /// Add a slot.
    Add { slot: u16 },
    /// Ending drag
    End,
}
impl From<QuickCraftStatus> for QuickCraftStatusKind {
    fn from(status: QuickCraftStatus) -> Self {
        match status {
            QuickCraftStatus::Start => QuickCraftStatusKind::Start,
            QuickCraftStatus::Add { .. } => QuickCraftStatusKind::Add,
            QuickCraftStatus::End => QuickCraftStatusKind::End,
        }
    }
}

/// Double click.
#[derive(Clone, Debug)]
pub struct PickupAllClick {
    /// The slot that we're double clicking on.
    ///
    /// It should be empty or at least not pickup-able (since the carried item
    /// is used as the filter).
    pub slot: u16,
    /// Impossible in vanilla clients.
    pub reversed: bool,
}
impl From<PickupAllClick> for ClickOperation {
    fn from(click: PickupAllClick) -> Self {
        ClickOperation::PickupAll(click)
    }
}

impl ClickOperation {
    /// Return the slot number that this operation is acting on, if any.
    ///
    /// Note that in the protocol, "None" is represented as -999.
    pub fn slot_num(&self) -> Option<u16> {
        match self {
            ClickOperation::Pickup(pickup) => match pickup {
                PickupClick::Left { slot } => *slot,
                PickupClick::Right { slot } => *slot,
                PickupClick::LeftOutside => None,
                PickupClick::RightOutside => None,
            },
            ClickOperation::QuickMove(quick_move) => match quick_move {
                QuickMoveClick::Left { slot } => Some(*slot),
                QuickMoveClick::Right { slot } => Some(*slot),
            },
            ClickOperation::Swap(swap) => Some(swap.source_slot),
            ClickOperation::Clone(clone) => Some(clone.slot),
            ClickOperation::Throw(throw) => match throw {
                ThrowClick::Single { slot } => Some(*slot),
                ThrowClick::All { slot } => Some(*slot),
            },
            ClickOperation::QuickCraft(quick_craft) => match quick_craft.status {
                QuickCraftStatus::Start => None,
                QuickCraftStatus::Add { slot } => Some(slot),
                QuickCraftStatus::End => None,
            },
            ClickOperation::PickupAll(pickup_all) => Some(pickup_all.slot),
        }
    }

    pub fn button_num(&self) -> u8 {
        match self {
            ClickOperation::Pickup(pickup) => match pickup {
                PickupClick::Left { .. } => 0,
                PickupClick::Right { .. } => 1,
                PickupClick::LeftOutside => 0,
                PickupClick::RightOutside => 1,
            },
            ClickOperation::QuickMove(quick_move) => match quick_move {
                QuickMoveClick::Left { .. } => 0,
                QuickMoveClick::Right { .. } => 1,
            },
            ClickOperation::Swap(swap) => swap.target_slot,
            ClickOperation::Clone(_) => 2,
            ClickOperation::Throw(throw) => match throw {
                ThrowClick::Single { .. } => 0,
                ThrowClick::All { .. } => 1,
            },
            ClickOperation::QuickCraft(quick_craft) => match quick_craft {
                QuickCraftClick {
                    kind: QuickCraftKind::Left,
                    status: QuickCraftStatus::Start,
                } => 0,
                QuickCraftClick {
                    kind: QuickCraftKind::Right,
                    status: QuickCraftStatus::Start,
                } => 4,
                QuickCraftClick {
                    kind: QuickCraftKind::Middle,
                    status: QuickCraftStatus::Start,
                } => 8,
                QuickCraftClick {
                    kind: QuickCraftKind::Left,
                    status: QuickCraftStatus::Add { .. },
                } => 1,
                QuickCraftClick {
                    kind: QuickCraftKind::Right,
                    status: QuickCraftStatus::Add { .. },
                } => 5,
                QuickCraftClick {
                    kind: QuickCraftKind::Middle,
                    status: QuickCraftStatus::Add { .. },
                } => 9,
                QuickCraftClick {
                    kind: QuickCraftKind::Left,
                    status: QuickCraftStatus::End,
                } => 2,
                QuickCraftClick {
                    kind: QuickCraftKind::Right,
                    status: QuickCraftStatus::End,
                } => 6,
                QuickCraftClick {
                    kind: QuickCraftKind::Middle,
                    status: QuickCraftStatus::End,
                } => 10,
            },
            ClickOperation::PickupAll(_) => 0,
        }
    }

    pub fn click_type(&self) -> ClickType {
        match self {
            ClickOperation::Pickup(_) => ClickType::Pickup,
            ClickOperation::QuickMove(_) => ClickType::QuickMove,
            ClickOperation::Swap(_) => ClickType::Swap,
            ClickOperation::Clone(_) => ClickType::Clone,
            ClickOperation::Throw(_) => ClickType::Throw,
            ClickOperation::QuickCraft(_) => ClickType::QuickCraft,
            ClickOperation::PickupAll(_) => ClickType::PickupAll,
        }
    }
}

#[derive(AzBuf, Clone, Copy, Debug, PartialEq)]
pub enum ClickType {
    Pickup = 0,
    QuickMove = 1,
    Swap = 2,
    Clone = 3,
    Throw = 4,
    QuickCraft = 5,
    PickupAll = 6,
}

impl Menu {
    /// Shift-click a slot in this menu.
    ///
    /// Keep in mind that this doesn't send any packets to the server, it just
    /// mutates this specific `Menu`.
    ///
    /// The target ranges and directions mirror each vanilla menu's
    /// `quickMoveStack`; moving out of a container into the player's
    /// inventory always walks the slots in reverse, which is what makes
    /// shift-clicked items land in the hotbar first. Moves that vanilla
    /// gates on data we don't have (smelting recipes, smithing templates,
    /// ...) are skipped instead of guessed: an unsimulated move is filled in
    /// by the server's corrections either way, while a wrong guess would
    /// make this simulation actively diverge from the real result.
    ///
    /// Like vanilla, this returns a copy of the original stack if the call
    /// moved anything (the caller loops on it for move-all semantics) and
    /// [`ItemStack::Empty`] otherwise.
    pub fn quick_move_stack(&mut self, slot_index: usize) -> ItemStack {
        let Some(original) = self.slot(slot_index).cloned() else {
            return ItemStack::Empty;
        };
        if original.is_empty() {
            return ItemStack::Empty;
        }

        let slot_location = self
            .location_for_slot(slot_index)
            .expect("we just checked to make sure the slot is Some above, so this shouldn't be able to error");
        match slot_location {
            MenuLocation::Player(l) => match l {
                PlayerMenuLocation::CraftResult => {
                    // vanilla's quickMoveStack runs onTake after a
                    // successful move: shift-crafting consumes the grid
                    if self.try_move_item_to_slots(slot_index, Player::INVENTORY_SLOTS, true) {
                        self.crafting_result_on_take(slot_index);
                    }
                }
                PlayerMenuLocation::Craft => {
                    self.try_move_item_to_slots(slot_index, Player::INVENTORY_SLOTS, false);
                }
                PlayerMenuLocation::Armor => {
                    self.try_move_item_to_slots(slot_index, Player::INVENTORY_SLOTS, false);
                }
                _ => {
                    // inventory, hotbar, and offhand slots: gear auto-equips
                    // into its empty equipment slot before anything else
                    let equip_slot_index = original
                        .as_present()
                        .and_then(equipment_slot_for_item)
                        .and_then(|equipment_slot| {
                            // helmet/chestplate/leggings/boots are slots 5-8
                            Some(match equipment_slot {
                                EquipmentSlot::Head => 5,
                                EquipmentSlot::Chest => 6,
                                EquipmentSlot::Legs => 7,
                                EquipmentSlot::Feet => 8,
                                EquipmentSlot::Offhand => Player::OFFHAND_SLOT,
                                _ => return None,
                            })
                        })
                        .filter(|&equip_slot_index| {
                            self.slot(equip_slot_index).unwrap().is_empty()
                        });

                    if let Some(equip_slot_index) = equip_slot_index {
                        self.try_move_item_to_slots(
                            slot_index,
                            equip_slot_index..=equip_slot_index,
                            false,
                        );
                    } else if l == PlayerMenuLocation::Inventory {
                        // shift-clicking in hotbar moves to inventory, and vice
                        // versa
                        if Player::is_hotbar_slot(slot_index) {
                            self.try_move_item_to_slots(
                                slot_index,
                                Player::INVENTORY_WITHOUT_HOTBAR_SLOTS,
                                false,
                            );
                        } else {
                            self.try_move_item_to_slots(slot_index, Player::HOTBAR_SLOTS, false);
                        }
                    } else {
                        // offhand
                        self.try_move_item_to_slots(slot_index, Player::INVENTORY_SLOTS, false);
                    }
                }
            },
            // plain containers: player items go into the container or
            // nowhere (vanilla has no inventory/hotbar fallback in these)
            MenuLocation::Generic9x1(l) => match l {
                Generic9x1MenuLocation::Contents => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                Generic9x1MenuLocation::Player => {
                    self.try_move_item_to_slots(slot_index, Menu::GENERIC9X1_CONTENTS_SLOTS, false);
                }
            },
            MenuLocation::Generic9x2(l) => match l {
                Generic9x2MenuLocation::Contents => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                Generic9x2MenuLocation::Player => {
                    self.try_move_item_to_slots(slot_index, Menu::GENERIC9X2_CONTENTS_SLOTS, false);
                }
            },
            MenuLocation::Generic9x3(l) => match l {
                Generic9x3MenuLocation::Contents => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                Generic9x3MenuLocation::Player => {
                    self.try_move_item_to_slots(slot_index, Menu::GENERIC9X3_CONTENTS_SLOTS, false);
                }
            },
            MenuLocation::Generic9x4(l) => match l {
                Generic9x4MenuLocation::Contents => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                Generic9x4MenuLocation::Player => {
                    self.try_move_item_to_slots(slot_index, Menu::GENERIC9X4_CONTENTS_SLOTS, false);
                }
            },
            MenuLocation::Generic9x5(l) => match l {
                Generic9x5MenuLocation::Contents => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                Generic9x5MenuLocation::Player => {
                    self.try_move_item_to_slots(slot_index, Menu::GENERIC9X5_CONTENTS_SLOTS, false);
                }
            },
            MenuLocation::Generic9x6(l) => match l {
                Generic9x6MenuLocation::Contents => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                Generic9x6MenuLocation::Player => {
                    self.try_move_item_to_slots(slot_index, Menu::GENERIC9X6_CONTENTS_SLOTS, false);
                }
            },
            MenuLocation::Generic3x3(l) => match l {
                Generic3x3MenuLocation::Contents => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                Generic3x3MenuLocation::Player => {
                    self.try_move_item_to_slots(slot_index, Menu::GENERIC3X3_CONTENTS_SLOTS, false);
                }
            },
            MenuLocation::Crafter3x3(l) => match l {
                Crafter3x3MenuLocation::Contents => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                Crafter3x3MenuLocation::Player => {
                    self.try_move_item_to_slots(slot_index, Menu::CRAFTER3X3_CONTENTS_SLOTS, false);
                }
            },
            MenuLocation::Anvil(l) => match l {
                AnvilMenuLocation::Result => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                AnvilMenuLocation::Player => {
                    // both anvil inputs accept any item (that's how renaming
                    // works), and vanilla has no inventory/hotbar fallback
                    // when they're occupied
                    self.try_move_item_to_slots(
                        slot_index,
                        Menu::ANVIL_FIRST_SLOT..=Menu::ANVIL_SECOND_SLOT,
                        false,
                    );
                }
                _ => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), false);
                }
            },
            MenuLocation::Beacon(l) => match l {
                BeaconMenuLocation::Payment => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                BeaconMenuLocation::Player => {
                    // a single payment item moves into the empty payment
                    // slot; the accepted kinds are the vanilla
                    // `beacon_payment_items` tag, which a datapack could
                    // change without us knowing
                    if original.count() == 1
                        && original
                            .as_present()
                            .is_some_and(|item| is_beacon_payment_item(item.kind))
                        && self.slot(Menu::BEACON_PAYMENT_SLOT).unwrap().is_empty()
                    {
                        self.try_move_item_to_slots(
                            slot_index,
                            Menu::BEACON_PAYMENT_SLOT..=Menu::BEACON_PAYMENT_SLOT,
                            false,
                        );
                    } else {
                        self.toggle_between_inventory_and_hotbar(slot_index);
                    }
                }
            },
            MenuLocation::BlastFurnace(l) => match l {
                BlastFurnaceMenuLocation::Result => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                BlastFurnaceMenuLocation::Player => {
                    // vanilla routes smeltables and fuels into the
                    // ingredient/fuel slots and toggles everything else
                    // between inventory and hotbar, but the gates are
                    // recipe-driven data we don't have
                }
                _ => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), false);
                }
            },
            MenuLocation::BrewingStand(l) => match l {
                BrewingStandMenuLocation::Player => {
                    // the knowable gates from vanilla's chain: blaze powder
                    // is fuel, and potions or bottles go
                    // into the one-item bottle slots. Anything else may
                    // or may not be a brewing ingredient, which is recipe
                    // data we don't have, so it stays put.
                    if original.kind() == ItemKind::BlazePowder {
                        self.try_move_item_to_slots(
                            slot_index,
                            Menu::BREWING_STAND_FUEL_SLOT..=Menu::BREWING_STAND_FUEL_SLOT,
                            false,
                        );
                    } else if is_potion_slot_item(original.kind()) {
                        self.try_move_item_to_slots(
                            slot_index,
                            Menu::BREWING_STAND_BOTTLES_SLOTS,
                            false,
                        );
                    }
                }
                _ => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
            },
            MenuLocation::Crafting(l) => match l {
                CraftingMenuLocation::Result => {
                    // vanilla's quickMoveStack runs onTake after a
                    // successful move: shift-crafting consumes the grid
                    if self.try_move_item_to_slots(slot_index, self.player_slots_range(), true) {
                        self.crafting_result_on_take(slot_index);
                    }
                }
                CraftingMenuLocation::Grid => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), false);
                }
                CraftingMenuLocation::Player => {
                    // vanilla tries the crafting grid first
                    self.try_move_item_to_slots_or_toggle_hotbar(
                        slot_index,
                        Menu::CRAFTING_GRID_SLOTS,
                    );
                }
            },
            MenuLocation::Enchantment(l) => match l {
                EnchantmentMenuLocation::Player => {
                    if original.kind() == ItemKind::LapisLazuli {
                        self.try_move_item_to_slots(
                            slot_index,
                            Menu::ENCHANTMENT_LAPIS_SLOT..=Menu::ENCHANTMENT_LAPIS_SLOT,
                            true,
                        );
                    } else if self.slot(Menu::ENCHANTMENT_ITEM_SLOT).unwrap().is_empty() {
                        // vanilla moves exactly one item into the
                        // enchanting slot, and has no inventory/hotbar
                        // fallback when it's occupied
                        let source = self.slot_mut(slot_index).unwrap();
                        let single = source.as_present_mut().unwrap().split(1);
                        source.update_empty();
                        *self.slot_mut(Menu::ENCHANTMENT_ITEM_SLOT).unwrap() = single.into();
                    }
                }
                _ => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
            },
            MenuLocation::Furnace(l) => match l {
                FurnaceMenuLocation::Result => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                FurnaceMenuLocation::Player => {
                    // vanilla routes smeltables and fuels into the
                    // ingredient/fuel slots and toggles everything else
                    // between inventory and hotbar, but the gates are
                    // recipe-driven data we don't have
                }
                _ => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), false);
                }
            },
            MenuLocation::Grindstone(l) => match l {
                GrindstoneMenuLocation::Result => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                GrindstoneMenuLocation::Player => {
                    // vanilla moves into the grindstone while either input
                    // is empty, and toggles between inventory and hotbar
                    // otherwise
                    if self.slot(Menu::GRINDSTONE_INPUT_SLOT).unwrap().is_empty()
                        || self
                            .slot(Menu::GRINDSTONE_ADDITIONAL_SLOT)
                            .unwrap()
                            .is_empty()
                    {
                        self.try_move_item_to_slots(
                            slot_index,
                            Menu::GRINDSTONE_INPUT_SLOT..=Menu::GRINDSTONE_ADDITIONAL_SLOT,
                            false,
                        );
                    } else {
                        self.toggle_between_inventory_and_hotbar(slot_index);
                    }
                }
                _ => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), false);
                }
            },
            MenuLocation::Hopper(l) => match l {
                HopperMenuLocation::Contents => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                HopperMenuLocation::Player => {
                    self.try_move_item_to_slots(slot_index, Menu::HOPPER_CONTENTS_SLOTS, false);
                }
            },
            // vanilla's lectern menu has no quick moves at all: the book
            // goes in by using it on the block and comes out through the
            // take-book button
            MenuLocation::Lectern(_) => {}
            MenuLocation::Loom(l) => match l {
                LoomMenuLocation::Result => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                LoomMenuLocation::Player => {
                    // vanilla routes banners, dyes, and patterns into their
                    // slots and toggles everything else between inventory
                    // and hotbar; we don't discriminate those item families
                    // yet, so the click stays unsimulated
                }
                _ => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), false);
                }
            },
            MenuLocation::Merchant(l) => match l {
                MerchantMenuLocation::Result => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                MerchantMenuLocation::Player => {
                    // vanilla never shift-clicks items into the payment
                    // slots (trades are filled by selecting them); it only
                    // toggles between inventory and hotbar
                    self.toggle_between_inventory_and_hotbar(slot_index);
                }
                _ => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), false);
                }
            },
            MenuLocation::ShulkerBox(l) => match l {
                ShulkerBoxMenuLocation::Contents => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                ShulkerBoxMenuLocation::Player => {
                    // like other plain containers there's no
                    // inventory/hotbar fallback, but items that can't live
                    // inside item containers don't move at all
                    if original
                        .as_present()
                        .is_some_and(|item| can_fit_inside_container_items(item.kind))
                    {
                        self.try_move_item_to_slots(
                            slot_index,
                            Menu::SHULKER_BOX_CONTENTS_SLOTS,
                            false,
                        );
                    }
                }
            },
            MenuLocation::Smithing(l) => match l {
                SmithingMenuLocation::Result => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                SmithingMenuLocation::Player => {
                    // vanilla gates player items on whether the smithing
                    // recipes accept them as template/base/addition, which
                    // is recipe data we don't have
                }
                _ => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), false);
                }
            },
            MenuLocation::Smoker(l) => match l {
                SmokerMenuLocation::Result => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                SmokerMenuLocation::Player => {
                    // vanilla routes smeltables and fuels into the
                    // ingredient/fuel slots and toggles everything else
                    // between inventory and hotbar, but the gates are
                    // recipe-driven data we don't have
                }
                _ => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), false);
                }
            },
            MenuLocation::CartographyTable(l) => match l {
                CartographyTableMenuLocation::Result => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                CartographyTableMenuLocation::Player => {
                    // vanilla: filled maps go into the map slot,
                    // paper-likes into the additional slot, everything else
                    // toggles between inventory and hotbar
                    if original
                        .as_present()
                        .is_some_and(|item| item.get_component::<components::MapId>().is_some())
                    {
                        self.try_move_item_to_slots(
                            slot_index,
                            Menu::CARTOGRAPHY_TABLE_MAP_SLOT..=Menu::CARTOGRAPHY_TABLE_MAP_SLOT,
                            false,
                        );
                    } else if matches!(
                        original.kind(),
                        ItemKind::Paper | ItemKind::Map | ItemKind::GlassPane
                    ) {
                        self.try_move_item_to_slots(
                            slot_index,
                            Menu::CARTOGRAPHY_TABLE_ADDITIONAL_SLOT
                                ..=Menu::CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
                            false,
                        );
                    } else {
                        self.toggle_between_inventory_and_hotbar(slot_index);
                    }
                }
                _ => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), false);
                }
            },
            MenuLocation::Stonecutter(l) => match l {
                StonecutterMenuLocation::Result => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), true);
                }
                StonecutterMenuLocation::Player => {
                    // vanilla moves items the stonecutter has recipes for
                    // into the input slot and toggles the rest — recipe
                    // data we don't have
                }
                _ => {
                    self.try_move_item_to_slots(slot_index, self.player_slots_range(), false);
                }
            },
        }

        // vanilla returns a copy of the original stack when the call moved
        // anything, and the click handler loops on that for its move-all
        // semantics; our unsimulated arms leave the slot untouched and so
        // return Empty here
        if self.slot(slot_index).unwrap().count() == original.count() {
            ItemStack::Empty
        } else {
            original
        }
    }

    /// Try moving an item to the given slots, and toggle it between the
    /// inventory and hotbar if nothing moved.
    ///
    /// This is the common tail of several vanilla menus' `quickMoveStack`.
    fn try_move_item_to_slots_or_toggle_hotbar(
        &mut self,
        slot_index: usize,
        target_slot_indexes: RangeInclusive<usize>,
    ) {
        if !self.try_move_item_to_slots(slot_index, target_slot_indexes, false) {
            self.toggle_between_inventory_and_hotbar(slot_index);
        }
    }

    /// Move an item from the player's main inventory to the hotbar, or the
    /// other way around, depending on where it currently is.
    fn toggle_between_inventory_and_hotbar(&mut self, slot_index: usize) {
        self.try_move_item_to_slots(
            slot_index,
            if self.is_hotbar_slot(slot_index) {
                self.player_slots_without_hotbar_range()
            } else {
                self.hotbar_slots_range()
            },
            false,
        );
    }

    /// Whether the slot is a menu's crafted/processed output. Vanilla backs
    /// every one of these with a result-slot class whose `mayPlace` refuses
    /// all items.
    pub fn is_result_slot(&self, slot_index: usize) -> bool {
        let Some(location) = self.location_for_slot(slot_index) else {
            return false;
        };
        matches!(
            location,
            MenuLocation::Player(PlayerMenuLocation::CraftResult)
                | MenuLocation::Anvil(AnvilMenuLocation::Result)
                | MenuLocation::BlastFurnace(BlastFurnaceMenuLocation::Result)
                | MenuLocation::CartographyTable(CartographyTableMenuLocation::Result)
                | MenuLocation::Crafting(CraftingMenuLocation::Result)
                | MenuLocation::Furnace(FurnaceMenuLocation::Result)
                | MenuLocation::Grindstone(GrindstoneMenuLocation::Result)
                | MenuLocation::Loom(LoomMenuLocation::Result)
                | MenuLocation::Merchant(MerchantMenuLocation::Result)
                | MenuLocation::Smithing(SmithingMenuLocation::Result)
                | MenuLocation::Smoker(SmokerMenuLocation::Result)
                | MenuLocation::Stonecutter(StonecutterMenuLocation::Result)
        )
    }

    /// Whether the given item could be placed in this menu.
    ///
    /// Result slots refuse everything. Vanilla's other placement
    /// restrictions (armor slots, furnace fuel, beacon payment, ...) are
    /// not modeled and default to allowing the placement.
    pub fn may_place(&self, target_slot_index: usize, _item: &ItemStackData) -> bool {
        !self.is_result_slot(target_slot_index)
    }

    /// Whether the item in the given slot could be clicked and picked up.
    ///
    /// TODO: right now this always returns true
    pub fn may_pickup(&self, _source_slot_index: usize) -> bool {
        true
    }

    /// The crafting side of vanilla's `ResultSlot.onTake`, which runs
    /// client-side as part of click prediction: taking a craft consumes one
    /// item from every occupied grid cell and leaves that ingredient's
    /// crafting remainder behind. Recipe-specific remainder overrides are
    /// server-only data, so like the vanilla client this uses the items'
    /// default remainders.
    ///
    /// No-op for every other slot, including the other result slots: their
    /// take effects (consuming furnace/trade/smithing inputs, ...) are not
    /// simulated.
    pub fn crafting_result_on_take(&mut self, result_slot_index: usize) {
        let grid = match self.location_for_slot(result_slot_index) {
            Some(MenuLocation::Player(PlayerMenuLocation::CraftResult)) => Player::CRAFT_SLOTS,
            Some(MenuLocation::Crafting(CraftingMenuLocation::Result)) => Menu::CRAFTING_GRID_SLOTS,
            _ => return,
        };
        for grid_slot_index in grid {
            let grid_slot = self.slot_mut(grid_slot_index).unwrap();
            let ItemStack::Present(grid_item) = grid_slot else {
                continue;
            };
            let remainder_kind = grid_item.kind.crafting_remainder();
            grid_slot.split(1);
            let Some(remainder_kind) = remainder_kind else {
                continue;
            };
            // the remainder stays in its grid cell when the cell emptied,
            // merges into a leftover matching stack, and otherwise moves to
            // the player inventory
            let mut remainder = ItemStackData::new(remainder_kind, 1);
            let grid_slot = self.slot_mut(grid_slot_index).unwrap();
            match grid_slot {
                ItemStack::Empty => *grid_slot = ItemStack::Present(remainder),
                ItemStack::Present(leftover)
                    if leftover.is_same_item_and_components(&remainder) =>
                {
                    remainder.count += leftover.count;
                    *grid_slot = ItemStack::Present(remainder);
                }
                _ => self.add_to_player_inventory(ItemStack::Present(remainder)),
            }
        }
    }

    /// Vanilla's `Inventory.add`: merge into partially filled matching
    /// stacks first, then place into a free slot, scanning the hotbar before
    /// the main rows (backing-inventory order, not menu order). Whatever
    /// finds no home is discarded, where vanilla drops it at the player's
    /// feet.
    pub fn add_to_player_inventory(&mut self, mut stack: ItemStack) {
        let hotbar_slots = self.hotbar_slots_range();
        let main_slots = *self.player_slots_range().start()..*hotbar_slots.start();
        let scan_order: Vec<usize> = hotbar_slots.chain(main_slots).collect();
        for &slot_index in &scan_order {
            if stack.is_empty() {
                return;
            }
            self.move_item_to_slot_if_stackable(&mut stack, slot_index);
        }
        for &slot_index in &scan_order {
            if stack.is_empty() {
                return;
            }
            self.move_item_to_slot_if_empty(&mut stack, slot_index);
        }
    }

    /// Whether the item in the slot can be picked up and placed.
    pub fn allow_modification(&self, target_slot_index: usize) -> bool {
        if !self.may_pickup(target_slot_index) {
            return false;
        }
        let item = self.slot(target_slot_index).unwrap();
        // the default here probably doesn't matter since we should only be
        // calling this if we already checked that the slot isn't empty
        item.as_present()
            .is_some_and(|item| self.may_place(target_slot_index, item))
    }

    /// Get the maximum number of items that can be placed in this slot.
    pub fn max_stack_size(&self, target_slot_index: usize) -> i32 {
        match self.location_for_slot(target_slot_index) {
            Some(MenuLocation::Player(PlayerMenuLocation::Armor))
            | Some(MenuLocation::Beacon(BeaconMenuLocation::Payment))
            | Some(MenuLocation::BrewingStand(BrewingStandMenuLocation::Bottles))
            | Some(MenuLocation::Enchantment(EnchantmentMenuLocation::Item))
            | Some(MenuLocation::Lectern(LecternMenuLocation::Book)) => 1,
            _ => 99,
        }
    }

    /// Try moving an item to a set of slots in this menu, walking the slots
    /// backwards when `reverse` is set.
    ///
    /// This mirrors vanilla's `AbstractContainerMenu.moveItemStackTo`: a
    /// first pass merges into existing matching stacks until the source runs
    /// out, then a second pass places the remainder into a single empty slot.
    /// Vanilla passes `reverse` for every move out of a container into the
    /// player's inventory, which is what makes shift-clicked items land in
    /// the hotbar first.
    ///
    /// Returns whether any items were moved.
    fn try_move_item_to_slots(
        &mut self,
        item_slot_index: usize,
        target_slot_indexes: RangeInclusive<usize>,
        reverse: bool,
    ) -> bool {
        let mut item_slot = self.slot(item_slot_index).unwrap().clone();
        let mut moved_anything = false;

        let order: Vec<usize> = if reverse {
            target_slot_indexes.rev().collect()
        } else {
            target_slot_indexes.collect()
        };

        // first see if we can stack it with other items
        if item_slot.as_present().is_some_and(|item| item.stackable()) {
            for &target_slot_index in &order {
                if item_slot.is_empty() {
                    break;
                }
                moved_anything |=
                    self.move_item_to_slot_if_stackable(&mut item_slot, target_slot_index);
            }
        }

        // and then put the remainder into the first empty slot that accepts
        // it (vanilla fills at most one empty slot per move)
        if item_slot.is_present() {
            for &target_slot_index in &order {
                if self.move_item_to_slot_if_empty(&mut item_slot, target_slot_index) {
                    moved_anything = true;
                    break;
                }
            }
        }

        *self.slot_mut(item_slot_index).unwrap() = item_slot;
        moved_anything
    }

    /// Merge this item slot into the target item slot, only if the target
    /// item slot holds a matching stack with room left: the target is topped
    /// up to its limit and the remainder stays in the source, following
    /// vanilla's arithmetic.
    ///
    /// Returns whether any items were moved.
    fn move_item_to_slot_if_stackable(
        &mut self,
        item_slot: &mut ItemStack,
        target_slot_index: usize,
    ) -> bool {
        let ItemStack::Present(item) = item_slot else {
            return false;
        };
        let target_count = {
            let ItemStack::Present(target_item) = self.slot(target_slot_index).unwrap() else {
                return false;
            };
            if !target_item.is_same_item_and_components(item) {
                return false;
            }
            target_item.count
        };

        let limit = i32::min(
            self.max_stack_size(target_slot_index),
            item.max_stack_size(),
        );
        if target_count >= limit {
            return false;
        }
        let moved = i32::min(item.count, limit - target_count);
        item.count -= moved;
        self.slot_mut(target_slot_index)
            .unwrap()
            .as_present_mut()
            .unwrap()
            .count = target_count + moved;
        item_slot.update_empty();
        true
    }

    /// Place the source stack into the target slot if that slot is empty and
    /// accepts it.
    ///
    /// Returns whether any items were moved.
    fn move_item_to_slot_if_empty(
        &mut self,
        source_item: &mut ItemStack,
        target_slot_index: usize,
    ) -> bool {
        let ItemStack::Present(source_item_data) = source_item else {
            return false;
        };
        let target_slot = self.slot(target_slot_index).unwrap();
        if !target_slot.is_empty() || !self.may_place(target_slot_index, source_item_data) {
            return false;
        }

        let limit = i32::min(
            self.max_stack_size(target_slot_index),
            source_item_data.max_stack_size(),
        );
        let new_target_slot_data =
            source_item_data.split(i32::min(limit, source_item_data.count) as u32);
        source_item.update_empty();

        let target_slot = self.slot_mut(target_slot_index).unwrap();
        *target_slot = new_target_slot_data.into();
        true
    }
}

/// The equipment slot an item would auto-equip into for a player, from its
/// `equippable` component (vanilla `Player.getEquipmentSlotForItem`).
///
/// Returns `None` for `Body` and `Saddle`, which are animal slots a player
/// can't wear.
fn equipment_slot_for_item(item: &ItemStackData) -> Option<EquipmentSlot> {
    let equippable = item.get_component::<components::Equippable>()?;
    if matches!(
        equippable.slot,
        EquipmentSlot::Mainhand | EquipmentSlot::Body | EquipmentSlot::Saddle
    ) {
        return None;
    }
    Some(equippable.slot)
}

/// Vanilla `Item.canFitInsideContainerItems`: shulker boxes refuse to nest
/// inside item containers.
fn can_fit_inside_container_items(kind: ItemKind) -> bool {
    !matches!(
        kind,
        ItemKind::ShulkerBox
            | ItemKind::WhiteShulkerBox
            | ItemKind::OrangeShulkerBox
            | ItemKind::MagentaShulkerBox
            | ItemKind::LightBlueShulkerBox
            | ItemKind::YellowShulkerBox
            | ItemKind::LimeShulkerBox
            | ItemKind::PinkShulkerBox
            | ItemKind::GrayShulkerBox
            | ItemKind::LightGrayShulkerBox
            | ItemKind::CyanShulkerBox
            | ItemKind::PurpleShulkerBox
            | ItemKind::BlueShulkerBox
            | ItemKind::BrownShulkerBox
            | ItemKind::GreenShulkerBox
            | ItemKind::RedShulkerBox
            | ItemKind::BlackShulkerBox
    )
}

/// The vanilla `beacon_payment_items` item tag. It's a data tag on the
/// server, so a datapack could change it without us knowing.
fn is_beacon_payment_item(kind: ItemKind) -> bool {
    matches!(
        kind,
        ItemKind::Emerald
            | ItemKind::Diamond
            | ItemKind::GoldIngot
            | ItemKind::IronIngot
            | ItemKind::NetheriteIngot
    )
}

/// Vanilla `BrewingStandMenu.PotionSlot.mayPlaceItem`: what the bottle
/// slots accept.
fn is_potion_slot_item(kind: ItemKind) -> bool {
    matches!(
        kind,
        ItemKind::Potion
            | ItemKind::SplashPotion
            | ItemKind::LingeringPotion
            | ItemKind::GlassBottle
    )
}
