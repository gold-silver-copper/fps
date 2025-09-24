use bevy::prelude::*;
pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, keyboard_input);
    }
}

/// Sends [`MovementAction`] events based on keyboard input.
fn keyboard_input(keyboard_input: Res<ButtonInput<KeyCode>>) {
    let quit = keyboard_input.any_pressed([KeyCode::KeyI]);
    if quit {
        panic!()
    }
}

#[derive(Bundle, Default)]
pub struct PlayerStuffBundle {
    pub inventory: PlayerInventory,
    pub stats: PlayerStats,
}

#[derive(Default, Component)]
pub struct PlayerInventory {
    pub bandages: u16,
    pub armor_bits: u16,
    pub grenades: u16,
    pub ninemm_ammo: u16,
}

#[derive(Component)]
pub struct PlayerStats {
    pub health: i16,
    pub armor: i16,
    pub max_health: i16,
    pub max_armor: i16,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            health: 100,
            armor: 100,
            max_health: 100,
            max_armor: 200,
        }
    }
}
