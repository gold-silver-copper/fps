use std::f32::consts::TAU;
use std::f32::consts::*;

use avian3d::{parry::shape::SharedShape, prelude::*};
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::mouse::MouseWheel;
use bevy::{input::mouse::MouseMotion, prelude::*};

use crate::GoldenControllerInput;

pub struct GunPlayPlugin;

impl Plugin for GunPlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (shoot_bullet, despawn_bullet));
    }
}

#[derive(Component)]
pub struct Bullet {}

/// System: when left mouse is clicked, spawn a bullet
fn shoot_bullet(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mouse: Res<ButtonInput<MouseButton>>,
    query: Query<(&GlobalTransform), With<Camera3d>>,
) {
    if mouse.pressed(MouseButton::Left) {
        if let Ok((global)) = query.single() {
            println!("OKAY");
            // Bullet spawn position = in front of player
            let forward = global.forward();
            let spawn_pos = global.translation() + forward * 1.0; // 1 unit in front

            // Bullet speed
            let speed = 800.0;
            // First, create an emissive material
            let emissive_material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.7, 0.6),
                emissive: LinearRgba::new(0.4, 0.3, 0.2, 0.1), // Glow color (usually darker than base)
                perceptual_roughness: 0.1,
                metallic: 0.8,
                ..default()
            });

            commands.spawn((
                // Small sphere collider
                Collider::sphere(0.01),
                Bullet {},
                Mesh3d(meshes.add(Sphere::new(0.01))),
                MeshMaterial3d(emissive_material),
                RigidBody::Dynamic,
                Mass(0.01),
                // Spawn at player position
                Transform::from_translation(spawn_pos),
                LinearVelocity(forward * speed),
                // Optional: disable gravity if you want straight shot
                GravityScale(1.0),
                // Optional: frictionless
                Friction::new(0.0),
                Restitution::new(0.99),
                LinearDamping(0.01),
            ));
        }
    }
}

/// System: when left mouse is clicked, spawn a bullet
fn despawn_bullet(mut commands: Commands, query: Query<(Entity, &LinearVelocity), With<Bullet>>) {
    for (e, v) in query.iter() {
        if v.length() < 50.0 {
            commands.entity(e).despawn();
            println!("DESPAWMD");
        }
    }
}
