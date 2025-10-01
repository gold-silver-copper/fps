use std::f32::consts::TAU;

use avian3d::prelude::*;
use bevy::{
    core_pipeline::{
        bloom::{Bloom, BloomCompositeMode},
        tonemapping::Tonemapping,
    },
    math::ops,
    prelude::*,
};
use bevy::{
    gltf::{Gltf, GltfMesh, GltfNode},
    math::Vec3Swizzles,
    render::camera::Exposure,
    window::CursorGrabMode,
};

use fps::*;
use iyes_perf_ui::{
    entries::{
        PerfUiFixedTimeEntries, PerfUiFramerateEntries, PerfUiSystemEntries, PerfUiWindowEntries,
    },
    prelude::PerfUiDefaultEntries,
    *,
};

const SPAWN_POINT: Vec3 = Vec3::new(0.0, 1.625, 0.0);

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 5000.0,
            affects_lightmapped_meshes: true,
        })
        .insert_resource(ClearColor(Color::linear_rgb(0.83, 0.96, 0.96)))
        .add_plugins((DefaultPlugins.set(ImagePlugin::default_nearest())))
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::SystemInformationDiagnosticsPlugin)
        .add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
        .add_plugins(iyes_perf_ui::PerfUiPlugin)
        .add_plugins(PhysicsPlugins::new(FixedPostUpdate))
        .add_plugins(GoldenUI)
        .add_plugins(GunPlayPlugin)
        .add_plugins(PhysicsDebugPlugin::default())
        .add_plugins(GoldenControllerPlugin)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(MyInputPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                manage_cursor,
                scene_colliders,
                //    display_text,
                respawn,
                rotate_this,
            ),
        )
        .run();
}

#[derive(Component)]
struct RotateThis {
    rotated: bool,
}

fn setup(
    mut commands: Commands,
    mut window: Query<&mut Window>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut window = window.single_mut().unwrap();
    window.title = String::from("Minimal FPS Controller Example");
    commands.spawn(PerfUiDefaultEntries::default());

    let e = commands
        .spawn((
            DirectionalLight {
                illuminance: light_consts::lux::CIVIL_TWILIGHT,
                shadows_enabled: true,
                ..default()
            },
            Transform::from_xyz(40.0, 20.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        ))
        .id();

    println!("light ent, {:#?}", e);

    // Note that we have two entities for the player
    // One is a "logical" player that handles the physics computation and collision
    // The other is a "render" player that is what is displayed to the user
    // This distinction is useful for later on if you want to add multiplayer,
    // where often time these two ideas are not exactly synced up
    let height = 1.0;
    let radius = 0.4;
    let mass = 80.0;

    let collidik = Collider::capsule(radius, height);
    let offsetik = collider_y_offset(&collidik);

    let body_entity = commands
        .spawn((
            collidik,
            Friction {
                dynamic_coefficient: 0.0,
                static_coefficient: 0.0,
                combine_rule: CoefficientCombine::Min,
            },
            Restitution {
                coefficient: 0.0,
                combine_rule: CoefficientCombine::Min,
            },
            LinearVelocity::ZERO,
            SpeculativeMargin::ZERO,
            RigidBody::Dynamic,
            Sleeping,
            LockedAxes::ROTATION_LOCKED,
            Mass(mass),
            GravityScale(1.0),
            Transform::from_translation(SPAWN_POINT),
            LogicalPlayer,
            LinearDamping(0.5),
        ))
        .insert(CameraConfig { height_offset: 0.0 })
        .insert(PlayerControllerBundle {
            controller: GoldenController {
                radius,
                height,
                mass,

                ..default()
            },
            ..default()
        })
        .insert(PlayerStuffBundle::default())
        .id();

    let feet_entity = commands
        .spawn((
            Collider::cylinder(10.0, 0.2),
            Transform::from_translation(SPAWN_POINT),
            FeetOf(body_entity), //     RigidBody::Kinematic,
        ))
        .id();

    let e = commands
        .spawn((
            Camera3d::default(),
            Camera {
                hdr: true, // 1. HDR is required for bloom
                //      clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            Tonemapping::TonyMcMapface, // 2. Using a tonemapper that desaturates to white is recommended
            Bloom::NATURAL,             // 3. Enable bloom for the camera
            Projection::Perspective(PerspectiveProjection {
                fov: TAU / 5.0,
                ..default()
            }),
            Exposure::SUNLIGHT,
            RenderPlayer {
                logical_entity: body_entity,
            },
        ))
        .id();
    println!("camera ent, {:#?}", e);
    commands.insert_resource(MainScene {
        handle: assets.load("playground3.glb"),
        is_loaded: false,
    });

    // A cube to move around
    commands.spawn((
        RigidBody::Dynamic,
        Collider::cuboid(1.0, 1.0, 1.0),
        Mesh3d(meshes.add(Cuboid::default())),
        Mass(40.0),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_translation(SPAWN_POINT + Vec3::new(10.0, 10.0, 10.0)),
        Friction {
            dynamic_coefficient: 0.9,
            static_coefficient: 0.9,
            combine_rule: CoefficientCombine::Max,
        },
    ));
}

fn rotate_this(mut query: Query<(&mut Transform, &mut RotateThis)>) {
    for (mut transform, mut rotate_this) in &mut query {
        if !rotate_this.rotated {
            transform.rotate_around(
                SPAWN_POINT + Vec3::new(2.75, -1.5, 3.0),
                Quat::from_euler(
                    EulerRot::XYZ,
                    45_f32.to_radians(),
                    0.0,
                    0.0, //30_f32.to_radians()
                ),
            );
            rotate_this.rotated = true;
        }
    }
}

fn respawn(mut query: Query<(&mut Transform, &mut LinearVelocity)>) {
    for (mut transform, mut velocity) in &mut query {
        if transform.translation.y > -50.0 {
            continue;
        }

        velocity.0 = Vec3::ZERO;
        transform.translation = SPAWN_POINT;
    }
}

#[derive(Resource)]
struct MainScene {
    handle: Handle<Gltf>,
    is_loaded: bool,
}

fn scene_colliders(
    mut commands: Commands,
    mut main_scene: ResMut<MainScene>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_mesh_assets: Res<Assets<GltfMesh>>,
    gltf_node_assets: Res<Assets<GltfNode>>,
) {
    if main_scene.is_loaded {
        return;
    }

    let gltf = gltf_assets.get(&main_scene.handle);

    if let Some(gltf) = gltf {
        let scene = gltf.scenes.first().unwrap().clone();
        commands.spawn(SceneRoot(scene));
        for node in &gltf.nodes {
            let node = gltf_node_assets.get(node).unwrap();
            if let Some(gltf_mesh) = node.mesh.clone() {
                let gltf_mesh = gltf_mesh_assets.get(&gltf_mesh).unwrap();
                for mesh_primitive in &gltf_mesh.primitives {
                    commands.spawn((
                        ColliderConstructor::TrimeshFromMeshWithConfig(
                            TrimeshFlags::FIX_INTERNAL_EDGES,
                        ),
                        Mesh3d(mesh_primitive.mesh.clone()),
                        RigidBody::Static,
                        node.transform,
                    ));
                }
            }
        }
        main_scene.is_loaded = true;
    }
}

fn manage_cursor(
    btn: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
    mut window_query: Query<&mut Window>,
    mut controller_query: Query<&mut GoldenController>,
) {
    for mut window in &mut window_query {
        if btn.just_pressed(MouseButton::Left) {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;
            for mut controller in &mut controller_query {
                controller.enable_input = true;
            }
        }
        if key.just_pressed(KeyCode::Escape) {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;
            for mut controller in &mut controller_query {
                controller.enable_input = false;
            }
        }
    }
}
