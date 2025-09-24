use bevy::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use ratatui::{
    prelude::{Stylize, Terminal},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use soft_ratatui::SoftBackend;
static FONT_DATA: &[u8] = include_bytes!("../assets/fira_mono.ttf");
pub struct GoldenUI;

impl Plugin for GoldenUI {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoftTerminal>()
            .add_systems(Startup, (setup_crosshair, setup_gun, ratatui_setup))
            .add_systems(Update, ui_example_system);
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
                GlobalZIndex(0), // This ensures it's always on top
                                 // BackgroundColor(ANTIQUE_WHITE.into()),
                                 // Outline::new(Val::Px(8.0), Val::ZERO, CRIMSON.into()),
            ));
        });
}

fn setup_gun(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load the crosshair PNG
    let gun = asset_server.load("AshesWeaponsV357/Graphics/FAL/FAL1D.png");
    //  let gun = asset_server.load("AshesWeaponsV357/Graphics/Glock/glock1.png");

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
                GlobalZIndex(0), // BackgroundColor(ANTIQUE_WHITE.into()),
                                 // Outline::new(Val::Px(8.0), Val::ZERO, CRIMSON.into()),
            ));
        });
}

// Marker if you want to reference it later
#[derive(Component)]
struct Crosshair;

// Create resource to hold the ratatui terminal
#[derive(Resource, Deref, DerefMut)]
struct SoftTerminal(Terminal<SoftBackend>);
impl Default for SoftTerminal {
    fn default() -> Self {
        let mut backend = SoftBackend::new_with_font(15, 15, 16, FONT_DATA);
        //backend.set_font_size(12);
        Self(Terminal::new(backend).unwrap())
    }
}

#[derive(Resource)]
struct MyRatatui(Handle<Image>);

// Render to the terminal and to egui , both are immediate mode
fn ui_example_system(
    mut softatui: ResMut<SoftTerminal>,
    mut images: ResMut<Assets<Image>>,
    my_handle: Res<MyRatatui>,
) {
    softatui
        .draw(|frame| {
            let area = frame.area();
            let textik = format!("Hello bevy! The window area is {}", area);
            frame.render_widget(
                Paragraph::new(textik)
                    .block(Block::new().title("Ratatui").borders(Borders::ALL))
                    .white()
                    .on_blue()
                    .wrap(Wrap { trim: false }),
                area,
            );
        })
        .expect("epic fail");
    println!("UPDATING");
    let width = softatui.backend().get_pixmap_width() as u32;
    let height = softatui.backend().get_pixmap_height() as u32;
    let data = softatui.backend().get_pixmap_data_as_rgba();

    let image = images.get_mut(&my_handle.0).expect("Image not found");
    *image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
}
fn ratatui_setup(
    mut commands: Commands,
    mut softatui: ResMut<SoftTerminal>,
    mut images: ResMut<Assets<Image>>,
) {
    let width = softatui.backend().get_pixmap_width() as u32;
    let height = softatui.backend().get_pixmap_height() as u32;
    let data = softatui.backend().get_pixmap_data_as_rgba();

    let image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    let handle = images.add(image);

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
                ImageNode::new(handle.clone()),
                Node {
                    height: Val::Percent(50.0), // 25% of screen height
                    //    width: Val::Percent(50.0),  // 25% of screen height
                    ..default()
                },
                GlobalZIndex(1), // BackgroundColor(ANTIQUE_WHITE.into()),
                                 // Outline::new(Val::Px(8.0), Val::ZERO, CRIMSON.into()),
            ));
        });
    commands.insert_resource(MyRatatui(handle));
}
