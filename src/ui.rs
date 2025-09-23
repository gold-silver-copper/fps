use bevy::{color::palettes::css::*, prelude::*, winit::WinitSettings};
use std::f32::consts::TAU;
use std::f32::consts::*;

use avian3d::{parry::shape::SharedShape, prelude::*};
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::mouse::MouseWheel;
use bevy::{input::mouse::MouseMotion, prelude::*};

pub struct GoldenUI;

impl Plugin for GoldenUI {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_crosshair));
    }
}

fn setup_crosshair(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load the crosshair PNG
    let crosshair = asset_server.load("crosshair2.png");

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(0.0),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                ImageNode::new(crosshair),
                Node {
                    width: Val::Px(27.0),
                    height: Val::Px(27.0),
                    ..default()
                },
                // BackgroundColor(ANTIQUE_WHITE.into()),
                // Outline::new(Val::Px(8.0), Val::ZERO, CRIMSON.into()),
            ));
        });
}

// Marker if you want to reference it later
#[derive(Component)]
struct Crosshair;
