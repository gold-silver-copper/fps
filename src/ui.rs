use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use embedded_graphics_unicodefonts::{
    mono_8x13_atlas, mono_8x13_bold_atlas, mono_8x13_italic_atlas,
};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Bar, Gauge};
use ratatui::{
    prelude::{Stylize, Terminal},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use soft_ratatui::{Bdf, CosmicText, EmbeddedGraphics, RgbPixmap, SoftBackend};

use crate::{GoldenControllerKeys, PlayerInventory, PlayerStats};

pub struct GoldenUI;
static FONTIK: &str = include_str!("../assets/cozette.bdf");

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

// Create resource to hold the ratatui terminal
#[derive(Resource, Deref, DerefMut)]
struct SoftTerminal(Terminal<SoftBackend<Bdf>>);
impl Default for SoftTerminal {
    fn default() -> Self {
        let font_regular = mono_8x13_atlas();
        let font_italic = mono_8x13_italic_atlas();
        let font_bold = mono_8x13_bold_atlas();
        let backend = SoftBackend::<Bdf>::new(100, 50, (6, 13), FONTIK);
        //backend.set_font_size(12);
        Self(Terminal::new(backend).unwrap())
    }
}
/*// Create resource to hold the ratatui terminal
#[derive(Resource, Deref, DerefMut)]
struct SoftTerminal(Terminal<SoftBackend<EmbeddedGraphics>>);
impl Default for SoftTerminal {
    fn default() -> Self {
        let font_regular = mono_8x13_atlas();
        let font_italic = mono_8x13_italic_atlas();
        let font_bold = mono_8x13_bold_atlas();
        let backend = SoftBackend::new(100, 50, font_regular, None, None);
        //backend.set_font_size(12);
        Self(Terminal::new(backend).unwrap())
    }
} */

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
    my_handle: Res<MyRatatui>,
    query: Query<(&PlayerStats, &PlayerInventory), With<GoldenControllerKeys>>,
) {
    if let Ok((stats, inv)) = query.single() {
        softatui
            .draw(|frame| {
                let area = frame.area();

                // Split the frame into two parts
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(0),    // Top part takes the rest
                        Constraint::Length(6), // Bottom part is 3 characters high
                    ])
                    .split(area);
                render_top_section(frame, chunks[0]);
                render_bottom_bar(frame, chunks[1]);
            })
            .expect("epic fail");

        let width = softatui.backend().get_pixmap_width() as u32;
        let height = softatui.backend().get_pixmap_height() as u32;
        let data = to_rgba_magenta_alpha(&softatui.backend().rgb_pixmap);

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

fn render_bottom_bar(frame: &mut Frame<'_>, chunk: ratatui::prelude::Rect) {
    // Split the frame into two parts
    let bar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 4); 4])
        .split(chunk);

    // Bottom part with border and text
    frame.render_widget(
        Gauge::default()
            .block(Block::bordered().border_type(ratatui::widgets::BorderType::Double))
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
        Paragraph::new("w░o▓of\n LOL")
            .white()
            .on_black()
            .wrap(Wrap { trim: false }),
        bar_chunks[2],
    );
    frame.render_widget(
        Gauge::default()
            .block(Block::bordered())
            .gauge_style(Color::Blue)
            .on_dark_gray()
            .ratio(50.0 / 100.0)
            .label("50/100"),
        bar_chunks[3],
    );
}
fn render_top_section(frame: &mut Frame<'_>, chunk: ratatui::prelude::Rect) {
    // Fill the top part with magenta
    frame.render_widget(
        Paragraph::new("").block(
            Block::new()
                .borders(Borders::NONE)
                .bg(Color::Rgb(MAGENTA.0, MAGENTA.1, MAGENTA.2)),
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
    let data = to_rgba_magenta_alpha(&softatui.backend().rgb_pixmap);

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

pub const MAGENTA: (u8, u8, u8) = (255, 0, 255);

pub fn to_rgba_magenta_alpha(mapik: &RgbPixmap) -> Vec<u8> {
    let mut rgba_data = Vec::with_capacity(mapik.width() * mapik.height() * 4);
    for chunk in mapik.data().chunks_exact(3) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        if (r, g, b) == MAGENTA {
            rgba_data.extend_from_slice(&[r, g, b, 0]);
        } else {
            rgba_data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    rgba_data
}
