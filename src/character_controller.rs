use avian3d::{math::*, prelude::*};
use bevy::ecs::query::Has;
use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MovementAction>()
            .insert_resource(QuakeMoveVars::default())
            .add_systems(FixedUpdate, (update_grounded, quake_movement).chain());
    }
}
#[derive(Resource)]
pub struct QuakeMoveVars {
    pub gravity: f32,
    pub stopspeed: f32,
    pub maxspeed: f32,
    pub accelerate: f32,
    pub airaccelerate: f32,
    pub friction: f32,
}

impl Default for QuakeMoveVars {
    fn default() -> Self {
        Self {
            gravity: 800.0,
            stopspeed: 100.0,
            maxspeed: 320.0,
            accelerate: 10.0,
            airaccelerate: 0.7,
            friction: 6.0,
        }
    }
}

/// An event sent for a movement input action.
#[derive(Event)]
pub enum MovementAction {
    Move(Vector2),
    Jump,
}

/// A marker component indicating that an entity is using a character controller.
#[derive(Component)]
pub struct CharacterController;

/// A marker component indicating that an entity is on the ground.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;
/// The acceleration used for character movement.
#[derive(Component)]
pub struct MovementAcceleration(Scalar);

/// The strength of a jump.
#[derive(Component)]
pub struct JumpImpulse(Scalar);

/// The maximum angle a slope can have for a character controller
/// to be able to climb and jump. If the slope is steeper than this angle,
/// the character will slide down.
#[derive(Component)]
pub struct MaxSlopeAngle(Scalar);

/// A bundle that contains the components needed for a basic
/// kinematic character controller.
#[derive(Bundle)]
pub struct CharacterControllerBundle {
    character_controller: CharacterController,
    body: RigidBody,
    collider: Collider,
    ground_caster: ShapeCaster,
    locked_axes: LockedAxes,
    movement: MovementBundle,
}

/// A bundle that contains components for character movement.
#[derive(Bundle)]
pub struct MovementBundle {
    acceleration: MovementAcceleration,

    jump_impulse: JumpImpulse,
    max_slope_angle: MaxSlopeAngle,
}

impl MovementBundle {
    pub const fn new(acceleration: Scalar, jump_impulse: Scalar, max_slope_angle: Scalar) -> Self {
        Self {
            acceleration: MovementAcceleration(acceleration),

            jump_impulse: JumpImpulse(jump_impulse),
            max_slope_angle: MaxSlopeAngle(max_slope_angle),
        }
    }
}

impl Default for MovementBundle {
    fn default() -> Self {
        Self::new(30.0, 7.0, PI * 0.45)
    }
}

impl CharacterControllerBundle {
    pub fn new(collider: Collider) -> Self {
        // Create shape caster as a slightly smaller version of collider
        let mut caster_shape = collider.clone();
        caster_shape.set_scale(Vector::ONE * 0.99, 10);

        Self {
            character_controller: CharacterController,
            body: RigidBody::Dynamic,
            collider,
            ground_caster: ShapeCaster::new(
                caster_shape,
                Vector::ZERO,
                Quaternion::default(),
                Dir3::NEG_Y,
            )
            .with_max_distance(0.2),
            locked_axes: LockedAxes::ROTATION_LOCKED,
            movement: MovementBundle::default(),
        }
    }

    pub fn with_movement(
        mut self,
        acceleration: Scalar,

        jump_impulse: Scalar,
        max_slope_angle: Scalar,
    ) -> Self {
        self.movement = MovementBundle::new(acceleration, jump_impulse, max_slope_angle);
        self
    }
}

#[derive(Debug, Component)]
pub struct WorldModelCamera;

#[derive(Debug, Component)]
pub struct Player;

#[derive(Debug, Component, Deref, DerefMut)]
pub struct CameraSensitivity(Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        Self(
            // These factors are just arbitrary mouse sensitivity values.
            // It's often nicer to have a faster horizontal sensitivity than vertical.
            // We use a component for them so that we can make them user-configurable at runtime
            // for accessibility reasons.
            // It also allows you to inspect them in an editor if you `Reflect` the component.
            Vec2::new(0.003, 0.002),
        )
    }
}

/// Updates the [`Grounded`] status for character controllers.
fn update_grounded(
    mut commands: Commands,
    mut query: Query<
        (Entity, &ShapeHits, &Rotation, Option<&MaxSlopeAngle>),
        With<CharacterController>,
    >,
) {
    for (entity, hits, rotation, max_slope_angle) in &mut query {
        // The character is grounded if the shape caster has a hit with a normal
        // that isn't too steep.
        let is_grounded = hits.iter().any(|hit| {
            if let Some(angle) = max_slope_angle {
                (rotation * -hit.normal2).angle_between(Vector::Y).abs() <= angle.0
            } else {
                true
            }
        });

        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

/// Responds to [`MovementAction`] events and moves character controllers accordingly.
fn movement(
    mut movement_event_reader: EventReader<MovementAction>,
    mut controllers: Query<(
        &MovementAcceleration,
        &JumpImpulse,
        &mut LinearVelocity,
        Has<Grounded>,
    )>,
    player: Single<(&Transform), With<Player>>,
) {
    //fixedupdate defaults to 64hz
    let delta_time = 1.0 / 64.0;

    let (transform) = player.into_inner();
    // let xyz = transform.rotation.xyz();
    for event in movement_event_reader.read() {
        for (movement_acceleration, jump_impulse, mut linear_velocity, is_grounded) in
            &mut controllers
        {
            match event {
                MovementAction::Move(direction) => {
                    // direction: Vec2 (x: right/left, y: forward/back)
                    let local_direction = Vec3::new(direction.x, 0.0, -direction.y);
                    let world_direction = transform.rotation * local_direction;

                    // Apply movement acceleration in the rotated direction
                    let acceleration = movement_acceleration.0 * delta_time;
                    linear_velocity.x += world_direction.x * acceleration;
                    linear_velocity.z += world_direction.z * acceleration;
                }
                MovementAction::Jump => {
                    if is_grounded {
                        linear_velocity.y = jump_impulse.0;
                    }
                }
            }
        }
    }
}

fn quake_movement(
    mut movement_event_reader: EventReader<MovementAction>,
    mut controllers: Query<
        (&mut LinearVelocity, &Transform, Has<Grounded>),
        With<CharacterController>,
    >,
    vars: Res<QuakeMoveVars>,
) {
    let dt = 1.0 / 64.0;

    for event in movement_event_reader.read() {
        for (mut vel, transform, grounded) in &mut controllers {
            let mut velocity = vel.0;

            // Build wishdir from input
            let wishdir = match event {
                MovementAction::Move(dir2d) => {
                    let forward = transform.forward().xz().normalize();
                    let right = transform.right().xz().normalize();
                    let wish = forward * -dir2d.y + right * dir2d.x;
                    if wish.length_squared() > 0.0 {
                        wish.normalize()
                    } else {
                        Vec2::ZERO
                    }
                }
                _ => Vec2::ZERO,
            };

            // Apply friction if grounded
            if grounded {
                let speed = velocity.length();
                if speed > 0.0 {
                    let control = speed.max(vars.stopspeed);
                    let drop = control * vars.friction * dt;
                    let newspeed = (speed - drop).max(0.0);
                    velocity *= newspeed / speed;
                }
            }

            match event {
                MovementAction::Move(dir2d) => {
                    if *dir2d != Vec2::ZERO {
                        let wishspeed = vars.maxspeed.min(dir2d.length() * vars.maxspeed);

                        if grounded {
                            // Ground accelerate
                            quake_accelerate(
                                &mut velocity,
                                wishdir,
                                wishspeed,
                                vars.accelerate,
                                dt,
                            );
                        } else {
                            // Air accelerate
                            quake_accelerate(
                                &mut velocity,
                                wishdir,
                                wishspeed,
                                vars.airaccelerate,
                                dt,
                            );
                        }
                    }
                }
                MovementAction::Jump => {
                    if grounded {
                        velocity.y = 270.0; // Quake jump impulse
                    }
                }
            }

            // Apply gravity
            if !grounded {
                velocity.y -= vars.gravity * dt;
            }

            vel.0 = velocity;
        }
    }
}

fn quake_accelerate(vel: &mut Vec3, wishdir: Vec2, wishspeed: f32, accel: f32, dt: f32) {
    let vel2d = vel.xz();
    let current_speed = vel2d.dot(wishdir);
    let add_speed = wishspeed - current_speed;
    if add_speed <= 0.0 {
        return;
    }
    let accel_speed = (accel * dt * wishspeed).min(add_speed);
    let push = wishdir * accel_speed;
    vel.x += push.x;
    vel.z += push.y;
}
