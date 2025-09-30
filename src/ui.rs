use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Bar, Gauge};
use ratatui::{
    prelude::{Stylize, Terminal},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use soft_ratatui::{Bdf, RgbPixmap, SoftBackend};

use crate::{GoldenControllerKeys, LogicalPlayer, PlayerInventory, PlayerStats};

pub struct GoldenUI;
static FONT_BDF: &str = include_str!("../assets/spleen-12x24.bdf");

impl Plugin for GoldenUI {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoftTerminal>()
            .add_systems(Startup, (setup_crosshair, setup_gun, ratatui_setup))
            .add_systems(Update, (ui_example_system, handle_resize_events));
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

#[derive(Resource, Deref, DerefMut)]
struct SoftTerminal(Terminal<SoftBackend<Bdf>>);
impl Default for SoftTerminal {
    fn default() -> Self {
        let backend = SoftBackend::<Bdf>::new(100, 50, (12, 24), FONT_BDF, None, None);
        //backend.set_font_size(12);
        Self(Terminal::new(backend).unwrap())
    }
}
#[derive(Resource)]
struct MyRatatui(Handle<Image>);

/// System that reacts to window resize
fn handle_resize_events(
    mut resize_reader: EventReader<WindowResized>,
    mut softatui: ResMut<SoftTerminal>,
) {
    for event in resize_reader.read() {
        let cur_pix_width = softatui.backend().char_width;
        let cur_pix_height = softatui.backend().char_height;
        let av_wid = (event.width / cur_pix_width as f32) as u16;
        let av_hei = (event.height / cur_pix_height as f32) as u16;
        softatui.backend_mut().resize(av_wid, av_hei);
    }
}
// Render to the terminal and to egui , both are immediate mode
fn ui_example_system(
    mut softatui: ResMut<SoftTerminal>,
    mut images: ResMut<Assets<Image>>,
    mut controller_query: Query<(&Transform, &LinearVelocity), With<LogicalPlayer>>,
    my_handle: Res<MyRatatui>,
    query: Query<(&PlayerStats, &PlayerInventory), With<GoldenControllerKeys>>,
) {
    let mut speed_text = String::new();
    for (transform, velocity) in &mut controller_query {
        speed_text = format!("spd: {:.2}", velocity.0.xz().length());
    }
    if let Ok((stats, inv)) = query.single() {
        softatui
            .draw(|frame| {
                let area = frame.area();

                // Split the frame into two parts
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(0),    // Top part takes the rest
                        Constraint::Length(1), // Bottom part is 3 characters high
                    ])
                    .split(area);
                render_top_section(frame, chunks[0]);
                render_bottom_bar(frame, chunks[1], speed_text);
            })
            .expect("epic fail");

        let width = softatui.backend().get_pixmap_width() as u32;
        let height = softatui.backend().get_pixmap_height() as u32;
        let data = softatui
            .backend()
            .rgb_pixmap
            .to_rgba_with_color_as_transparent(&(255, 0, 255));

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
}

fn render_bottom_bar(frame: &mut Frame<'_>, chunk: ratatui::prelude::Rect, speed_text: String) {
    // Split the frame into two parts
    let bar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 4); 4])
        .split(chunk);

    // Bottom part with border and text
    frame.render_widget(
        Gauge::default()
            .block(Block::new())
            .gauge_style(Color::Blue)
            .on_dark_gray()
            .ratio(50.0 / 100.0)
            .label("50/100"),
        bar_chunks[0],
    );
    // Bottom part with border and text
    frame.render_widget(
        Paragraph::new(
            "mAAAAAeow\nAAAAAAAA\nAAAAAAAAabcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPRSTUWYXZQ",
        )
        .white()
        .on_green()
        .wrap(Wrap { trim: false }),
        bar_chunks[1],
    );
    // Bottom part with border and text
    frame.render_widget(
        Paragraph::new("w░░░░░o▓▓▓▓of\n LOL")
            .white()
            .on_black()
            .wrap(Wrap { trim: false }),
        bar_chunks[2],
    );
    frame.render_widget(Paragraph::new(speed_text).black(), bar_chunks[3]);
}
fn render_top_section(frame: &mut Frame<'_>, chunk: ratatui::prelude::Rect) {
    // Fill the top part with magenta
    frame.render_widget(
        Paragraph::new("").block(
            Block::new()
                .borders(Borders::NONE)
                .bg(Color::Rgb(255, 0, 255)),
        ),
        chunk,
    );
}

fn ratatui_setup(
    mut commands: Commands,
    mut softatui: ResMut<SoftTerminal>,
    mut images: ResMut<Assets<Image>>,
) {
    let width = softatui.backend().get_pixmap_width() as u32;
    let height = softatui.backend().get_pixmap_height() as u32;
    let data = softatui
        .backend()
        .rgb_pixmap
        .to_rgba_with_color_as_transparent(&(255, 0, 255));

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
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexEnd,
            align_items: AlignItems::Center,
            row_gap: Val::Px(0.0),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                ImageNode::new(handle.clone()),
                Node {
                    //   height: Val::Percent(50.0), // 25% of screen height
                    //    width: Val::Percent(50.0),  // 25% of screen height
                    ..default()
                },
                GlobalZIndex(1), // BackgroundColor(ANTIQUE_WHITE.into()),
                                 // Outline::new(Val::Px(8.0), Val::ZERO, CRIMSON.into()),
            ));
        });
    commands.insert_resource(MyRatatui(handle));
}
