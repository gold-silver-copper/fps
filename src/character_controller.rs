use std::f32::consts::*;

use avian3d::{
    parry::{math::Point, shape::SharedShape},
    prelude::*,
};
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::mouse::MouseWheel;
use bevy::{
    input::mouse::MouseMotion,
    math::{Vec3Swizzles, VectorSpace},
    prelude::*,
};

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
pub static FPS: f64 = 120.0;
impl Plugin for FpsControllerPlugin {
    fn build(&self, app: &mut App) {
        use bevy::input::{gamepad, keyboard, mouse, touch};

        app.insert_resource(Time::<Fixed>::from_hz(FPS))
            .add_systems(
                PreUpdate,
                (
                    fps_controller_input,
                    fps_controller_look,
                    fps_controller_render,
                    scroll_events,
                )
                    .chain()
                    .after(mouse::mouse_button_input_system)
                    .after(keyboard::keyboard_input_system)
                    .after(gamepad::gamepad_event_processing_system)
                    .after(gamepad::gamepad_connection_system)
                    .after(touch::touch_screen_input_system),
            )
            .add_systems(FixedUpdate, fps_controller_move);
    }
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
    pub jump: bool,
    pub crouch: bool,
    pub pitch: f32,
    pub yaw: f32,
    pub movement: Vec3,
    pub lean: f32, // -1.0 left, +1.0 right
    pub lean_degree_mod: f32,
    pub crouch_degree_mod: f32,
}

#[derive(Component)]
pub struct FpsController {
    pub radius: f32,

    /// If the distance to the ground is less than this value, the player is considered grounded
    pub grounded_distance: f32,
    pub walk_speed: f32,

    pub forward_speed: f32,
    pub side_speed: f32,
    pub air_speed_cap: f32,

    pub air_acceleration: f32,

    pub acceleration: f32,
    pub crouched_speed: f32,
    pub crouch_speed: f32,
    /// If the dot product (alignment) of the normal of the surface and the upward vector,
    /// which is a value from [-1, 1], is greater than this value, ground movement is applied
    pub traction_normal_cutoff: f32,

    pub height: f32,
    pub ground_tick: u8,
    pub jump_tick: u8,
    pub pitch: f32,
    pub yaw: f32,
    pub friction: f32,
    pub mass: f32,
    pub lean_degree: f32,

    pub sensitivity: f32,
    pub enable_input: bool,
    pub crouch_degree: f32,
    pub jump_force: f32,
    pub lean_max: f32,

    pub key_forward: KeyCode,
    pub key_back: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,

    pub air_friction: f32,
    pub lean_side_impulse: f32,
    pub leaning_speed: f32,

    pub key_lean_left: KeyCode,
    pub key_lean_right: KeyCode,
    pub key_crouch: KeyCode,
    pub key_jump: KeyCode,
}

impl Default for FpsController {
    fn default() -> Self {
        Self {
            grounded_distance: 0.04,
            radius: 0.5,

            walk_speed: 7.0,
            mass: 80.0,
            crouched_speed: 3.5,
            crouch_speed: 4.0,

            air_friction: 0.1,
            jump_force: 6.0,

            forward_speed: 30.0,
            side_speed: 30.0,
            air_speed_cap: 2.0,

            air_acceleration: 10.0,
            crouch_degree: 1.0,
            lean_max: 0.45,
            leaning_speed: 2.0,

            ground_tick: 0,
            jump_tick: 0,
            height: 1.8,
            lean_degree: 0.0,

            acceleration: 3.5,

            traction_normal_cutoff: 0.6,
            friction: 0.9,

            pitch: 0.0,
            yaw: 0.0,
            lean_side_impulse: 60.0,

            enable_input: true,
            key_forward: KeyCode::KeyW,
            key_back: KeyCode::KeyS,
            key_left: KeyCode::KeyA,
            key_right: KeyCode::KeyD,
            key_lean_left: KeyCode::KeyQ,
            key_lean_right: KeyCode::KeyE,
            key_crouch: KeyCode::ShiftLeft,
            key_jump: KeyCode::Space,
            //  key_movement_mod_up:KeyCode::M
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

const SLIGHT_SCALE_DOWN: f32 = 0.7;

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
            &mut ExternalImpulse,
            &mut Friction,
        ),
        With<LogicalPlayer>,
    >,
) {
    let dt = 1.0 / FPS as f32;

    for (
        entity,
        input,
        mut controller,
        mut collider,
        mut transform,
        mut velocity,
        mut external_force,
        mut friction,
    ) in query.iter_mut()
    {
        // Shape cast downwards to find ground
        // Better than a ray cast as it handles when you are near the edge of a surface
        let filter = SpatialQueryFilter::default().with_excluded_entities([entity]);
        let some_hit = spatial_query_pipeline.cast_shape(
            // Consider when the controller is right up against a wall
            // We do not want the shape cast to detect it,
            // so provide a slightly smaller collider in the XZ plane
            &scaled_collider_laterally(&collider, SLIGHT_SCALE_DOWN),
            transform.translation,
            transform.rotation,
            -Dir3::Y,
            //hack to stay grounded while leaning
            &ShapeCastConfig::from_max_distance(
                controller.grounded_distance + controller.lean_degree.abs() / 20.0,
            ),
            &filter,
        );

        let scale_vec = Vec3::splat(controller.mass);

        let speeds = Vec3::new(controller.side_speed, 0.0, controller.forward_speed);
        let mut move_to_world = Mat3::from_axis_angle(Vec3::Y, input.yaw);
        move_to_world.z_axis *= -1.0; // Forward is -Z
        let mut wish_direction = move_to_world * (input.movement * speeds);
        let mut wish_speed = wish_direction.length();
        if wish_speed > f32::EPSILON {
            // Avoid division by zero
            wish_direction /= wish_speed; // Effectively normalize, avoid length computation twice
        }
        // limit move speed while leaning or crouching
        let max_speed = if input.crouch || controller.lean_degree.abs() > 0.05 {
            controller.crouched_speed
        } else {
            controller.walk_speed
        };
        wish_speed = f32::min(wish_speed, max_speed);

        // LEAN
        // Always start with base yaw rotation
        let yaw_rotation = Quat::from_euler(EulerRot::YXZ, input.yaw, 0.0, 0.0);
        let right_dir = yaw_rotation * Vec3::X; // local +X is "right"
        let old_degree = controller.lean_degree;
        let mut degree_change = 0.0;
        let lean_change = controller.leaning_speed * dt;
        if input.lean.abs() > 0.1 {
            controller.lean_degree += input.lean * lean_change;
            let lean_mod = 1.0 - input.lean_degree_mod;
            controller.lean_degree = controller
                .lean_degree
                .clamp(-1.0 * lean_mod, 1.0 * lean_mod);

            degree_change = controller.lean_degree - old_degree;
        } else {
            // Relax back to neutral
            if controller.lean_degree.abs() < lean_change * 1.5 {
                controller.lean_degree = 0.0;
            } else {
                controller.lean_degree -= controller.lean_degree.signum() * lean_change;
                degree_change = controller.lean_degree - old_degree;
            }
        }
        //shift collider to facilitate looking around walls
        transform.translation += right_dir * controller.lean_side_impulse * degree_change * dt;
        // How much to lean
        let lean_amount = controller.lean_degree * controller.lean_max;
        let lean_rotation = Quat::from_axis_angle(Vec3::Z, -lean_amount);
        transform.rotation = (yaw_rotation * lean_rotation).normalize();

        match some_hit {
            // NEAR GROUND
            Some(hit) => {
                //High Friction for controllable character
                friction.dynamic_coefficient = controller.friction;
                friction.static_coefficient = controller.friction;

                let jump_force = Vec3 {
                    x: 0.0,
                    y: -0.07,
                    z: 0.0,
                };
                external_force.apply_impulse(jump_force * scale_vec);

                friction.combine_rule = CoefficientCombine::Average;
                // check if player is on walkable slope
                let has_traction =
                    Vec3::dot(hit.normal1, Vec3::Y) > controller.traction_normal_cutoff;

                if !input.jump {
                    // This is for walking up slopes well
                    wish_direction =
                        wish_direction - hit.normal1 * Vec3::dot(wish_direction, hit.normal1);
                    let add = acceleration(
                        wish_direction,
                        wish_speed,
                        controller.acceleration,
                        velocity.0,
                        dt,
                    );
                    external_force.apply_impulse(add * scale_vec);
                } else {
                    //When bhopping with jump held, use air accel logic for smoother movement
                    wish_speed = f32::min(wish_speed, controller.air_speed_cap);
                    let add = acceleration(
                        wish_direction,
                        wish_speed,
                        controller.air_acceleration,
                        velocity.0,
                        dt,
                    );
                    //  println!("ADD IS {:#?}", add);
                    external_force.apply_impulse(add * scale_vec);
                }

                if has_traction {
                    //This fixes bug that pushes player randomly upon landing
                    let linear_velocity = velocity.0;
                    let normal_force = Vec3::dot(linear_velocity, hit.normal1) * hit.normal1;
                    velocity.0 -= normal_force;

                    if input.jump && controller.jump_tick > 1 && controller.ground_tick > 1 {
                        let jump_force = Vec3 {
                            x: 0.0,
                            y: controller.jump_force,
                            z: 0.0,
                        };
                        external_force.apply_impulse(jump_force * scale_vec);
                        controller.jump_tick = 0;
                    } else {
                        controller.jump_tick = controller.jump_tick.saturating_add(1);
                    }
                }
                controller.ground_tick = controller.ground_tick.saturating_add(1);
            }
            //IN AIR
            None => {
                controller.ground_tick = 0;

                friction.dynamic_coefficient = controller.air_friction;
                friction.static_coefficient = controller.air_friction;
                friction.combine_rule = CoefficientCombine::Min;
                wish_speed = f32::min(wish_speed, controller.air_speed_cap);
                //   println!("WISH DIR IS {:#?}", wish_direction);

                let add = acceleration(
                    wish_direction,
                    wish_speed,
                    controller.air_acceleration,
                    velocity.0,
                    dt,
                );
                //  println!("ADD IS {:#?}", add);
                external_force.apply_impulse(add * scale_vec);
            }
        }
        /* Crouching */
        if input.crouch {
            controller.crouch_degree += controller.crouch_speed * dt;
        } else {
            controller.crouch_degree -= controller.crouch_speed * dt;
        }
        controller.crouch_degree = controller
            .crouch_degree
            .clamp(1.0, 2.0 - input.crouch_degree_mod);

        collider.set_shape(SharedShape::cylinder(
            (controller.height / 2.0) / (controller.crouch_degree),
            controller.radius,
        ));
    }
}

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
            0.0,
            get_axis(&key_input, controller.key_forward, controller.key_back),
        );
        input.lean = get_axis(
            &key_input,
            controller.key_lean_right,
            controller.key_lean_left,
        );

        input.jump = key_input.pressed(controller.key_jump);
        input.crouch = key_input.pressed(controller.key_crouch);
    }
}

fn scroll_events(
    mut evr_scroll: EventReader<MouseWheel>,
    mut query: Query<(&FpsController, &mut FpsControllerInput)>,
) {
    let mut mod_shift = 0.0;

    for ev in evr_scroll.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                println!(
                    "Scroll (line units): vertical: {}, horizontal: {}",
                    ev.y, ev.x
                );
                if ev.y.abs() > 0.1 {
                    mod_shift += ev.y.signum() * 0.1;
                }
                if ev.x.abs() > 0.1 {
                    mod_shift += ev.x.signum() * 0.1;
                }
            }
            MouseScrollUnit::Pixel => {
                println!(
                    "Scroll (pixel units): vertical: {}, horizontal: {}",
                    ev.y, ev.x
                );
                if ev.y.abs() > 0.1 {
                    mod_shift += ev.y.signum() * 0.1;
                }
                if ev.x.abs() > 0.1 {
                    mod_shift += ev.x.signum() * 0.1;
                }
            }
        }
        println!("MOD SHIFT {:#?}", mod_shift);
    }
    mod_shift = mod_shift.clamp(-1.0, 1.0);

    for (controller, mut input) in query
        .iter_mut()
        .filter(|(controller, _)| controller.enable_input)
    {
        if input.lean.abs() > 0.1 {
            input.lean_degree_mod += mod_shift;
            input.lean_degree_mod = input.lean_degree_mod.clamp(0.0, 1.0);
        } else if input.crouch {
            input.crouch_degree_mod += mod_shift;
            input.crouch_degree_mod = input.crouch_degree_mod.clamp(0.0, 1.0);
        }
    }
}

pub fn fps_controller_look(mut query: Query<(&mut FpsController, &FpsControllerInput)>) {
    for (mut controller, input) in query.iter_mut() {
        controller.pitch = input.pitch;
        controller.yaw = input.yaw;
    }
}

/// Returns the offset that puts a point at the center of the player transform to the bottom of the collider.
/// Needed for when we want to originate something at the foot of the player.
fn collider_y_offset(collider: &Collider) -> Vec3 {
    Vec3::Y
        * if let Some(cylinder) = collider.shape().as_cylinder() {
            cylinder.half_height
        } else {
            panic!("Controller must use a cylinder or capsule collider")
        }
}

/// Return a collider that is scaled laterally (XZ plane) but not vertically (Y axis).
fn scaled_collider_laterally(collider: &Collider, scale: f32) -> Collider {
    if let Some(cylinder) = collider.shape().as_cylinder() {
        let new_cylinder = Collider::cylinder(cylinder.radius * scale, cylinder.half_height * 2.0);
        new_cylinder
    } else {
        panic!("Controller must use a cylinder or capsule collider")
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
            // just copy logical rotation instead of rebuilding
            /*let zetik = logical_transform.rotation.xyz().z;

            */
            let pitch_quat = Quat::from_euler(EulerRot::YXZ, 0.0, controller.pitch, 0.0);
            render_transform.rotation = logical_transform.rotation.mul_quat(pitch_quat);
        }
    }
}
