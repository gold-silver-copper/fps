use bevy::prelude::*;

pub struct GoldenUI;

impl Plugin for GoldenUI {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_crosshair, setup_gun));
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

fn setup_gun(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load the crosshair PNG
    let gun = asset_server.load("AshesWeaponsV357/Graphics/FAL/FAL1D.png");
    let gun2 = asset_server.load("AshesWeaponsV357/Graphics/Glock/glock1.png");

    // Force nearest-neighbor (pixel-perfect) sampling

    commands
        .spawn(Node {
            width: Val::Percent(80.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexEnd,
            align_items: AlignItems::FlexEnd,
            row_gap: Val::Px(0.0),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                ImageNode::new(gun),
                Node {
                    height: Val::Percent(50.0), // 25% of screen height
                    //    width: Val::Percent(50.0),  // 25% of screen height
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
