use std::f32::consts::*;

use avian3d::{
    parry::{math::Point, shape::SharedShape},
    prelude::*,
};
use bevy::{input::mouse::MouseMotion, math::Vec3Swizzles, prelude::*};

/// Manages the FPS controllers. Executes in `PreUpdate`, after bevy's internal
/// input processing is finished.
///
/// If you need a system in `PreUpdate` to execute after FPS Controller's systems,
/// Do it like so:
///
/// ```
/// # use bevy::prelude::*;
///
/// struct MyPlugin;
/// impl Plugin for MyPlugin {
///     fn build(&self, app: &mut App) {
///         app.add_systems(
///             PreUpdate,
///             my_system.after(bevy_fps_controller::controller::fps_controller_render),
///         );
///     }
/// }
///
/// fn my_system() { }
/// ```
pub struct FpsControllerPlugin;

pub static FPS: f64 = 96.0;

impl Plugin for FpsControllerPlugin {
    fn build(&self, app: &mut App) {
        use bevy::input::{gamepad, keyboard, mouse, touch};

        app.add_systems(
            PreUpdate,
            (
                fps_controller_input,
                fps_controller_look,
                fps_controller_render,
            )
                .chain()
                .after(mouse::mouse_button_input_system)
                .after(keyboard::keyboard_input_system)
                .after(gamepad::gamepad_event_processing_system)
                .after(gamepad::gamepad_connection_system)
                .after(touch::touch_screen_input_system),
        )
        .insert_resource(Time::<Fixed>::from_hz(FPS))
        .add_systems(FixedUpdate, (fps_controller_move));
    }
}

#[derive(PartialEq)]
pub enum MoveMode {
    Noclip,
    Ground,
}

#[derive(Component)]
pub struct LogicalPlayer;

#[derive(Component)]
pub struct RenderPlayer {
    pub logical_entity: Entity,
}

#[derive(Component)]
pub struct CameraConfig {
    pub height_offset: f32,
}

#[derive(Component, Default)]
pub struct FpsControllerInput {
    pub fly: bool,
    pub sprint: bool,
    pub jump: bool,
    pub crouch: bool,
    pub pitch: f32,
    pub yaw: f32,
    pub movement: Vec3,
}

#[derive(Component)]
pub struct FpsController {
    pub move_mode: MoveMode,
    pub radius: f32,

    /// If the distance to the ground is less than this value, the player is considered grounded
    pub grounded_distance: f32,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub forward_speed: f32,
    pub side_speed: f32,
    pub air_speed_cap: f32,
    pub air_acceleration: f32,
    pub max_air_speed: f32,
    pub acceleration: f32,

    /// If the dot product (alignment) of the normal of the surface and the upward vector,
    /// which is a value from [-1, 1], is greater than this value, ground movement is applied
    pub traction_normal_cutoff: f32,
    pub friction_speed_cutoff: f32,
    pub jump_speed: f32,
    pub fly_speed: f32,
    pub crouched_speed: f32,
    pub crouch_speed: f32,
    pub uncrouch_speed: f32,
    pub height: f32,
    pub upright_height: f32,
    pub crouch_height: f32,
    pub fast_fly_speed: f32,
    pub fly_friction: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub ground_tick: u8,

    pub sensitivity: f32,
    pub enable_input: bool,

    pub key_forward: KeyCode,
    pub key_back: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_up: KeyCode,
    pub key_down: KeyCode,
    pub key_sprint: KeyCode,
    pub key_jump: KeyCode,
    pub key_fly: KeyCode,
    pub key_crouch: KeyCode,
}

impl Default for FpsController {
    fn default() -> Self {
        Self {
            move_mode: MoveMode::Ground,
            grounded_distance: 0.125,
            radius: 0.5,
            fly_speed: 10.0,
            fast_fly_speed: 30.0,

            walk_speed: 9.0,
            run_speed: 14.0,
            forward_speed: 30.0,
            side_speed: 30.0,
            air_speed_cap: 2.0,
            air_acceleration: 20.0,
            max_air_speed: 15.0,
            crouched_speed: 5.0,
            crouch_speed: 6.0,
            uncrouch_speed: 8.0,
            height: 3.0,
            upright_height: 3.0,
            crouch_height: 1.5,
            acceleration: 10.0,

            traction_normal_cutoff: 0.7,
            friction_speed_cutoff: 0.1,
            fly_friction: 0.5,
            pitch: 0.0,
            yaw: 0.0,
            ground_tick: 0,

            jump_speed: 4.0,

            enable_input: true,
            key_forward: KeyCode::KeyW,
            key_back: KeyCode::KeyS,
            key_left: KeyCode::KeyA,
            key_right: KeyCode::KeyD,
            key_up: KeyCode::KeyQ,
            key_down: KeyCode::KeyE,
            key_sprint: KeyCode::ShiftLeft,
            key_jump: KeyCode::Space,
            key_fly: KeyCode::KeyF,
            key_crouch: KeyCode::ControlLeft,
            sensitivity: 0.001,
        }
    }
}

// ██╗      ██████╗  ██████╗ ██╗ ██████╗
// ██║     ██╔═══██╗██╔════╝ ██║██╔════╝
// ██║     ██║   ██║██║  ███╗██║██║
// ██║     ██║   ██║██║   ██║██║██║
// ███████╗╚██████╔╝╚██████╔╝██║╚██████╗
// ╚══════╝ ╚═════╝  ╚═════╝ ╚═╝ ╚═════╝

// Used as padding by camera pitching (up/down) to avoid spooky math problems
const ANGLE_EPSILON: f32 = 0.001953125;

const SLIGHT_SCALE_DOWN: f32 = 0.9375;

pub fn fps_controller_input(
    key_input: Res<ButtonInput<KeyCode>>,
    mut mouse_events: EventReader<MouseMotion>,
    mut query: Query<(&FpsController, &mut FpsControllerInput)>,
) {
    for (controller, mut input) in query
        .iter_mut()
        .filter(|(controller, _)| controller.enable_input)
    {
        let mut mouse_delta = Vec2::ZERO;
        for mouse_event in mouse_events.read() {
            mouse_delta += mouse_event.delta;
        }
        mouse_delta *= controller.sensitivity;

        input.pitch = (input.pitch - mouse_delta.y)
            .clamp(-FRAC_PI_2 + ANGLE_EPSILON, FRAC_PI_2 - ANGLE_EPSILON);
        input.yaw -= mouse_delta.x;
        if input.yaw.abs() > PI {
            input.yaw = input.yaw.rem_euclid(TAU);
        }

        input.movement = Vec3::new(
            get_axis(&key_input, controller.key_right, controller.key_left),
            get_axis(&key_input, controller.key_up, controller.key_down),
            get_axis(&key_input, controller.key_forward, controller.key_back),
        );
        input.sprint = key_input.pressed(controller.key_sprint);
        input.jump = key_input.pressed(controller.key_jump);
        input.fly = key_input.just_pressed(controller.key_fly);
        input.crouch = key_input.pressed(controller.key_crouch);
    }
}

pub fn fps_controller_look(mut query: Query<(&mut FpsController, &FpsControllerInput)>) {
    for (mut controller, input) in query.iter_mut() {
        controller.pitch = input.pitch;
        controller.yaw = input.yaw;
    }
}

pub fn fps_controller_move(
    spatial_query_pipeline: Res<SpatialQueryPipeline>,
    mut query: Query<
        (
            Entity,
            &FpsControllerInput,
            &mut FpsController,
            &mut Collider,
            &mut Transform,
            &mut LinearVelocity,
        ),
        With<LogicalPlayer>,
    >,
) {
    let dt = 1.0 / FPS as f32;

    for (entity, input, mut controller, mut collider, mut transform, mut velocity) in
        query.iter_mut()
    {
        if input.fly {
            controller.move_mode = match controller.move_mode {
                MoveMode::Noclip => MoveMode::Ground,
                MoveMode::Ground => MoveMode::Noclip,
            }
        }

        match controller.move_mode {
            MoveMode::Noclip => {
                if input.movement == Vec3::ZERO {
                    let friction = controller.fly_friction.clamp(0.0, 1.0);
                    velocity.0 *= 1.0 - friction;
                    if velocity.length_squared() < f32::EPSILON {
                        velocity.0 = Vec3::ZERO;
                    }
                } else {
                    let fly_speed = if input.sprint {
                        controller.fast_fly_speed
                    } else {
                        controller.fly_speed
                    };
                    let mut move_to_world =
                        Mat3::from_euler(EulerRot::YXZ, input.yaw, input.pitch, 0.0);
                    move_to_world.z_axis *= -1.0; // Forward is -Z
                    move_to_world.y_axis = Vec3::Y; // Vertical movement aligned with world up
                    velocity.0 = move_to_world * input.movement * fly_speed;
                }
            }
            MoveMode::Ground => {
                let speeds = Vec3::new(controller.side_speed, 0.0, controller.forward_speed);
                let mut move_to_world = Mat3::from_axis_angle(Vec3::Y, input.yaw);
                move_to_world.z_axis *= -1.0; // Forward is -Z
                let mut wish_direction = move_to_world * (input.movement * speeds);
                let mut wish_speed = wish_direction.length();
                if wish_speed > f32::EPSILON {
                    // Avoid division by zero
                    wish_direction /= wish_speed; // Effectively normalize, avoid length computation twice
                }
                let max_speed = if input.crouch {
                    controller.crouched_speed
                } else if input.sprint {
                    controller.run_speed
                } else {
                    controller.walk_speed
                };
                wish_speed = f32::min(wish_speed, max_speed);

                // Shape cast downwards to find ground
                // Better than a ray cast as it handles when you are near the edge of a surface
                let filter = SpatialQueryFilter::default().with_excluded_entities([entity]);
                if let Some(hit) = spatial_query_pipeline.cast_shape(
                    // Consider when the controller is right up against a wall
                    // We do not want the shape cast to detect it,
                    // so provide a slightly smaller collider in the XZ plane
                    &scaled_collider_laterally(&collider, SLIGHT_SCALE_DOWN),
                    transform.translation,
                    transform.rotation,
                    -Dir3::Y,
                    &ShapeCastConfig::from_max_distance(controller.grounded_distance),
                    &filter,
                ) {
                    let has_traction =
                        Vec3::dot(hit.normal1, Vec3::Y) > controller.traction_normal_cutoff;

                    let add = acceleration(
                        wish_direction,
                        wish_speed,
                        controller.acceleration,
                        velocity.0,
                        dt,
                    );

                    velocity.0 += add;

                    if has_traction {
                        let linear_velocity = velocity.0;
                        velocity.0 -= Vec3::dot(linear_velocity, hit.normal1) * hit.normal1;

                        if input.jump {
                            velocity.0.y = controller.jump_speed;
                        }
                    }

                    // Increment ground tick but cap at max value
                    controller.ground_tick = controller.ground_tick.saturating_add(1);
                } else {
                    controller.ground_tick = 0;
                    wish_speed = f32::min(wish_speed, controller.air_speed_cap);

                    let add = acceleration(
                        wish_direction,
                        wish_speed,
                        controller.air_acceleration,
                        velocity.0,
                        dt,
                    );

                    velocity.0 += add;

                    let air_speed = velocity.xz().length();
                    if air_speed > controller.max_air_speed {
                        let ratio = controller.max_air_speed / air_speed;
                        velocity.0.x *= ratio;
                        velocity.0.z *= ratio;
                    }
                };

                /* Crouching */

                let crouch_height = controller.crouch_height;
                let upright_height = controller.upright_height;

                let crouch_speed = if input.crouch {
                    -controller.crouch_speed
                } else {
                    controller.uncrouch_speed
                };
                controller.height += dt * crouch_speed;
                controller.height = controller.height.clamp(crouch_height, upright_height);

                if let Some(cylinder) = collider.shape().as_cylinder() {
                    let radius = cylinder.radius;
                    collider.set_shape(SharedShape::cylinder(controller.height * 0.5, radius));
                } else {
                    panic!("Controller must use a cylinder collider")
                }
            }
        }
    }
}

/// Returns the offset that puts a point at the center of the player transform to the bottom of the collider.
/// Needed for when we want to originate something at the foot of the player.
fn collider_y_offset(collider: &Collider) -> Vec3 {
    Vec3::Y
        * if let Some(cylinder) = collider.shape().as_cylinder() {
            cylinder.half_height
        } else {
            panic!("Controller must use a cylinder collider")
        }
}

/// Return a collider that is scaled laterally (XZ plane) but not vertically (Y axis).
fn scaled_collider_laterally(collider: &Collider, scale: f32) -> Collider {
    if let Some(cylinder) = collider.shape().as_cylinder() {
        let new_cylinder = Collider::cylinder(cylinder.radius * scale, cylinder.half_height * 2.0);
        new_cylinder
    } else {
        panic!("Controller must use a cylinder collider")
    }
}

fn acceleration(
    wish_direction: Vec3,
    wish_speed: f32,
    acceleration: f32,
    velocity: Vec3,
    dt: f32,
) -> Vec3 {
    let velocity_projection = Vec3::dot(velocity, wish_direction);
    let add_speed = wish_speed - velocity_projection;
    if add_speed <= 0.0 {
        return Vec3::ZERO;
    }

    let acceleration_speed = f32::min(acceleration * wish_speed * dt, add_speed);
    wish_direction * acceleration_speed
}

fn get_pressed(key_input: &Res<ButtonInput<KeyCode>>, key: KeyCode) -> f32 {
    if key_input.pressed(key) { 1.0 } else { 0.0 }
}

fn get_axis(key_input: &Res<ButtonInput<KeyCode>>, key_pos: KeyCode, key_neg: KeyCode) -> f32 {
    get_pressed(key_input, key_pos) - get_pressed(key_input, key_neg)
}

// ██████╗ ███████╗███╗   ██╗██████╗ ███████╗██████╗
// ██╔══██╗██╔════╝████╗  ██║██╔══██╗██╔════╝██╔══██╗
// ██████╔╝█████╗  ██╔██╗ ██║██║  ██║█████╗  ██████╔╝
// ██╔══██╗██╔══╝  ██║╚██╗██║██║  ██║██╔══╝  ██╔══██╗
// ██║  ██║███████╗██║ ╚████║██████╔╝███████╗██║  ██║
// ╚═╝  ╚═╝╚══════╝╚═╝  ╚═══╝╚═════╝ ╚══════╝╚═╝  ╚═╝

pub fn fps_controller_render(
    mut render_query: Query<(&mut Transform, &RenderPlayer), With<RenderPlayer>>,
    logical_query: Query<
        (&Transform, &Collider, &FpsController, &CameraConfig),
        (With<LogicalPlayer>, Without<RenderPlayer>),
    >,
) {
    for (mut render_transform, render_player) in render_query.iter_mut() {
        if let Ok((logical_transform, collider, controller, camera_config)) =
            logical_query.get(render_player.logical_entity)
        {
            let collider_offset = collider_y_offset(collider);
            let camera_offset = Vec3::Y * camera_config.height_offset;
            render_transform.translation =
                logical_transform.translation + collider_offset + camera_offset;
            render_transform.rotation =
                Quat::from_euler(EulerRot::YXZ, controller.yaw, controller.pitch, 0.0);
        }
    }
}
