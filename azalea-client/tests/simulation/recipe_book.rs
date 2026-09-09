use azalea_client::{
    inventory::{PlaceRecipeEvent, RecipeBook},
    test_utils::prelude::*,
};
use azalea_protocol::{
    common::recipe::{
        ItemSlotDisplay, RecipeDisplayData, RecipeDisplayId, ShapelessCraftingRecipeDisplay,
        SlotDisplayData,
    },
    packets::{
        ConnectionProtocol,
        game::{
            ClientboundPlaceGhostRecipe, ClientboundRecipeBookAdd, ClientboundRecipeBookRemove,
            ClientboundRecipeBookSettings, ServerboundGamePacket,
            c_recipe_book_add::{Entry, RecipeDisplayEntry},
            c_recipe_book_settings::RecipeBookSettings,
        },
    },
};
use azalea_registry::builtin::{ItemKind, RecipeBookCategory};

fn recipe_entry(id: u32, result: ItemKind) -> RecipeDisplayEntry {
    let display = |item| SlotDisplayData::Item(ItemSlotDisplay { item });
    RecipeDisplayEntry {
        id: RecipeDisplayId(id),
        display: RecipeDisplayData::Shapeless(ShapelessCraftingRecipeDisplay {
            ingredients: vec![display(ItemKind::OakLog)],
            result: display(result),
            crafting_station: display(ItemKind::CraftingTable),
        }),
        group: None,
        category: RecipeBookCategory::CraftingBuildingBlocks,
        crafting_requirements: None,
    }
}

#[test]
fn recipe_book_packets_converge_and_stale_old_recipes() {
    let _lock = init();
    let mut simulation = Simulation::new(ConnectionProtocol::Game);
    simulation.receive_packet(default_login_packet());
    simulation.tick();

    let entry = recipe_entry(300, ItemKind::OakPlanks);
    simulation.receive_packet(ClientboundRecipeBookAdd {
        entries: vec![Entry {
            contents: entry.clone(),
            flags: 2,
        }],
        replace: true,
    });
    simulation.tick();

    let recipe_book = simulation.component::<RecipeBook>();
    let old_recipe = recipe_book.recipe(RecipeDisplayId(300)).unwrap();
    assert_eq!(old_recipe.result_item(), Some(ItemKind::OakPlanks));
    assert!(recipe_book.is_highlighted(RecipeDisplayId(300)));

    let settings = RecipeBookSettings {
        gui_open: true,
        filtering_craftable: true,
        ..Default::default()
    };
    simulation.receive_packet(ClientboundRecipeBookSettings {
        book_settings: settings.clone(),
    });
    simulation.receive_packet(ClientboundPlaceGhostRecipe {
        container_id: 0,
        recipe: entry.display.clone(),
    });
    simulation.tick();
    let recipe_book = simulation.component::<RecipeBook>();
    assert_eq!(recipe_book.settings(), &settings);
    assert_eq!(recipe_book.ghost_recipe().unwrap().recipe, entry.display);

    simulation.receive_packet(ClientboundRecipeBookRemove {
        recipes: vec![RecipeDisplayId(300)],
    });
    simulation.tick();
    let recipe_book = simulation.component::<RecipeBook>();
    assert!(!recipe_book.contains(&old_recipe));
    assert!(recipe_book.recipe(RecipeDisplayId(300)).is_none());
}

#[test]
fn place_recipe_uses_the_current_numeric_display_id() {
    let _lock = init();
    let mut simulation = Simulation::new(ConnectionProtocol::Game);
    simulation.receive_packet(default_login_packet());
    simulation.tick();

    simulation.receive_packet(ClientboundRecipeBookAdd {
        entries: vec![Entry {
            contents: recipe_entry(300, ItemKind::OakPlanks),
            flags: 0,
        }],
        replace: true,
    });
    simulation.tick();
    let recipe = simulation
        .component::<RecipeBook>()
        .recipe(RecipeDisplayId(300))
        .unwrap();
    let menu_generation = simulation
        .component::<azalea_client::inventory::InventorySyncState>()
        .menu_generation();
    let sent_packets = SentPackets::new(&mut simulation);

    simulation.trigger(PlaceRecipeEvent {
        entity: simulation.entity,
        container_id: 0,
        menu_generation,
        recipe,
        use_max_items: false,
    });
    simulation.update();

    sent_packets.expect("PlaceRecipe", |packet| {
        matches!(
            packet,
            ServerboundGamePacket::PlaceRecipe(packet)
                if packet.container_id == 0
                    && packet.recipe == RecipeDisplayId(300)
                    && !packet.shift_down
        )
    });
}
