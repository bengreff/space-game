mod common;

use sunscatter_app::save::SaveGame;

#[test]
fn default_save_loads() {
    let save = SaveGame::load_from_file("default")
        .expect("data/saves/default/save.ron should deserialize without error");
    assert!(
        save.simulation_time >= 0.0,
        "simulation_time should be non-negative, got {}",
        save.simulation_time
    );
}

#[test]
fn default_save_has_name() {
    let save = SaveGame::load_from_file("default").unwrap();
    assert!(
        !save.name.is_empty(),
        "Save game name should not be empty"
    );
}
