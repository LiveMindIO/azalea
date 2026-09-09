use azalea_chat::FormattedText;
use azalea_client::{
    inventory::{ConfirmedContainerClickEvent, ContainerClickEvent, InventorySyncState},
    test_utils::prelude::*,
};
use azalea_core::position::ChunkPos;
use azalea_entity::inventory::Inventory;
use azalea_inventory::{ItemStack, operations::PickupClick};
use azalea_protocol::packets::{
    ConnectionProtocol,
    game::{
        ClientboundContainerClose, ClientboundContainerSetContent, ClientboundContainerSetSlot,
        ClientboundOpenScreen, ClientboundSetChunkCacheCenter, ClientboundSetCursorItem,
        ClientboundSetPlayerInventory, ServerboundGamePacket,
    },
};
use azalea_registry::builtin::{ItemKind, MenuKind};

/// The server heals a diverged client model with `set_cursor_item`,
/// `set_player_inventory`, and `container_set_content` (which re-baselines
/// the menu *including* the carried stack and `state_id`, like vanilla's
/// `initializeContents`). All three must actually be applied — a dropped
/// correction makes every simulation divergence permanent.
#[test]
fn test_inventory_corrections_are_applied() {
    let _lock = init();

    let mut simulation = Simulation::new(ConnectionProtocol::Game);
    simulation.receive_packet(default_login_packet());
    simulation.tick();
    // receive a chunk so the player is "loaded" now
    simulation.receive_packet(ClientboundSetChunkCacheCenter { x: 1, z: 23 });
    simulation.receive_packet(make_basic_empty_chunk(
        ChunkPos::new(1, 23),
        (384 + 64) / 16,
    ));
    simulation.tick();

    // set_cursor_item corrects the carried stack
    let dirt = ItemStack::new(ItemKind::Dirt, 5);
    simulation.receive_packet(ClientboundSetCursorItem {
        contents: dirt.clone(),
    });
    simulation.tick();
    simulation.with_component(|inventory: &Inventory| {
        assert_eq!(
            inventory.carried, dirt,
            "set_cursor_item must correct the carried stack"
        );
    });

    // set_player_inventory corrects a slot through vanilla Inventory
    // addressing: 0-8 is the hotbar
    let stone = ItemStack::new(ItemKind::Stone, 7);
    simulation.receive_packet(ClientboundSetPlayerInventory {
        slot: 2,
        contents: stone.clone(),
    });
    simulation.tick();
    simulation.with_component(|inventory: &Inventory| {
        let hotbar_start = *inventory.menu().hotbar_slots_range().start();
        assert_eq!(
            inventory.menu().slot(hotbar_start + 2),
            Some(&stone),
            "set_player_inventory slot 2 must write hotbar slot 2"
        );
    });

    // container_set_content for window 0 re-baselines the player inventory
    // including carried_item and state_id
    let planks = ItemStack::new(ItemKind::SprucePlanks, 3);
    let carried = ItemStack::new(ItemKind::Diamond, 2);
    let mut items = vec![ItemStack::Empty; 46];
    items[9] = planks.clone();
    simulation.receive_packet(ClientboundContainerSetContent {
        container_id: 0,
        state_id: 5,
        items,
        carried_item: carried.clone(),
    });
    simulation.tick();
    simulation.with_component(|inventory: &Inventory| {
        assert_eq!(inventory.inventory_menu.slot(9), Some(&planks));
        assert_eq!(
            inventory.carried, carried,
            "container_set_content must apply carried_item"
        );
        assert_eq!(
            inventory.state_id, 5,
            "container_set_content must apply state_id"
        );
    });
    simulation.with_component(|sync_state: &InventorySyncState| {
        assert_eq!(sync_state.authoritative_revision(), 3);
        assert!(sync_state.is_initialized(0));
    });
}

#[test]
fn player_slots_and_state_ids_stay_synchronized_while_a_container_is_open() {
    let _lock = init();
    let mut simulation = Simulation::new(ConnectionProtocol::Game);
    simulation.receive_packet(default_login_packet());
    simulation.tick();

    simulation.receive_packet(ClientboundContainerSetContent {
        container_id: 0,
        state_id: 5,
        items: vec![ItemStack::Empty; 46],
        carried_item: ItemStack::Empty,
    });
    simulation.receive_packet(ClientboundOpenScreen {
        container_id: 1,
        menu_type: MenuKind::Crafting,
        title: FormattedText::default(),
    });
    simulation.receive_packet(ClientboundContainerSetContent {
        container_id: 1,
        state_id: 7,
        items: vec![ItemStack::Empty; 46],
        carried_item: ItemStack::Empty,
    });
    simulation.tick();

    let stone = ItemStack::new(ItemKind::Stone, 3);
    simulation.receive_packet(ClientboundContainerSetSlot {
        container_id: 0,
        state_id: 6,
        slot: 9,
        item_stack: stone.clone(),
    });
    simulation.tick();
    simulation.with_component(|inventory: &Inventory| {
        assert_eq!(
            inventory.state_id, 7,
            "the open menu keeps its own state ID"
        );
        assert_eq!(inventory.inventory_menu.slot(9), Some(&stone));
        let open_player_start = *inventory.menu().player_slots_range().start();
        assert_eq!(inventory.menu().slot(open_player_start), Some(&stone));
    });

    simulation.receive_packet(ClientboundContainerClose { container_id: 1 });
    simulation.tick();
    simulation.with_component(|inventory: &Inventory| {
        assert_eq!(inventory.id, 0);
        assert_eq!(
            inventory.state_id, 6,
            "closing restores the player-menu state ID"
        );
        assert_eq!(inventory.inventory_menu.slot(9), Some(&stone));
    });
    simulation.with_component(|sync_state: &InventorySyncState| {
        assert!(sync_state.is_initialized(0));
    });
}

#[test]
fn full_content_correction_reverts_a_rejected_click_and_rebaselines_state() {
    let _lock = init();
    let mut simulation = Simulation::new(ConnectionProtocol::Game);
    simulation.receive_packet(default_login_packet());
    simulation.tick();
    simulation.receive_packet(ClientboundSetChunkCacheCenter { x: 1, z: 23 });
    simulation.receive_packet(make_basic_empty_chunk(
        ChunkPos::new(1, 23),
        (384 + 64) / 16,
    ));
    simulation.tick();

    let stone = ItemStack::new(ItemKind::Stone, 10);
    let mut contents = vec![ItemStack::Empty; 46];
    contents[9] = stone.clone();
    simulation.receive_packet(ClientboundContainerSetContent {
        container_id: 0,
        state_id: 5,
        items: contents.clone(),
        carried_item: ItemStack::Empty,
    });
    simulation.tick();

    let sent_packets = SentPackets::new(&mut simulation);
    simulation.trigger(ContainerClickEvent {
        entity: simulation.entity,
        window_id: 0,
        operation: PickupClick::Left { slot: Some(9) }.into(),
    });
    simulation.update();
    sent_packets.expect("ContainerClick with state 5", |packet| {
        matches!(packet, ServerboundGamePacket::ContainerClick(packet) if packet.state_id == 5)
    });
    simulation.with_component(|inventory: &Inventory| {
        assert_eq!(inventory.inventory_menu.slot(9), Some(&ItemStack::Empty));
        assert_eq!(inventory.carried, stone);
    });

    simulation.receive_packet(ClientboundContainerSetContent {
        container_id: 0,
        state_id: 6,
        items: contents,
        carried_item: ItemStack::Empty,
    });
    simulation.tick();
    simulation.with_component(|inventory: &Inventory| {
        assert_eq!(inventory.inventory_menu.slot(9), Some(&stone));
        assert_eq!(inventory.carried, ItemStack::Empty);
        assert_eq!(inventory.state_id, 6);
    });

    sent_packets.clear();
    simulation.trigger(ContainerClickEvent {
        entity: simulation.entity,
        window_id: 0,
        operation: PickupClick::Left { slot: Some(9) }.into(),
    });
    simulation.update();
    sent_packets.expect("ContainerClick with corrected state 6", |packet| {
        matches!(packet, ServerboundGamePacket::ContainerClick(packet) if packet.state_id == 6)
    });
}

#[test]
fn confirmed_click_forces_a_resync_and_fences_the_menu_generation() {
    let _lock = init();
    let mut simulation = Simulation::new(ConnectionProtocol::Game);
    simulation.receive_packet(default_login_packet());
    simulation.tick();

    let stone = ItemStack::new(ItemKind::Stone, 10);
    let mut contents = vec![ItemStack::Empty; 46];
    contents[9] = stone.clone();
    simulation.receive_packet(ClientboundContainerSetContent {
        container_id: 0,
        state_id: 5,
        items: contents,
        carried_item: ItemStack::Empty,
    });
    simulation.tick();

    let mut generation = 0;
    simulation.with_component(|sync_state: &InventorySyncState| {
        generation = sync_state.menu_generation();
    });
    let sent_packets = SentPackets::new(&mut simulation);
    simulation.trigger(ConfirmedContainerClickEvent {
        entity: simulation.entity,
        window_id: 0,
        menu_generation: generation,
        operation: PickupClick::Left { slot: Some(9) }.into(),
    });
    simulation.update();
    sent_packets.expect("ContainerClick with deliberately stale state", |packet| {
        matches!(packet, ServerboundGamePacket::ContainerClick(packet) if packet.state_id == (5 ^ 0x4000))
    });
    simulation.with_component(|inventory: &Inventory| {
        assert_eq!(inventory.inventory_menu.slot(9), Some(&ItemStack::Empty));
        assert_eq!(inventory.carried, stone);
    });

    sent_packets.clear();
    simulation.trigger(ConfirmedContainerClickEvent {
        entity: simulation.entity,
        window_id: 0,
        menu_generation: generation.wrapping_add(1),
        operation: PickupClick::Left { slot: Some(9) }.into(),
    });
    simulation.update();
    sent_packets.expect_empty();
}
