use crate::character_controller::*;
use avian3d::math::*;

use bevy::prelude::*;
pub struct MyInputPlugin;

impl Plugin for MyInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, keyboard_input);
    }
}

/// Sends [`MovementAction`] events based on keyboard input.
fn keyboard_input(keyboard_input: Res<ButtonInput<KeyCode>>) {
    let quit = keyboard_input.any_pressed([KeyCode::KeyQ]);
    if quit {
        panic!()
    }
}
