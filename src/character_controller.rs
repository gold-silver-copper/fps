use std::f32::consts::TAU;
use std::f32::consts::*;

use avian3d::{parry::shape::SharedShape, prelude::*};
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::mouse::MouseWheel;
use bevy::{input::mouse::MouseMotion, prelude::*};

pub struct GoldenControllerPlugin;
pub static FPS: f64 = 120.0;
impl Plugin for GoldenControllerPlugin {
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
            .add_systems(
                FixedUpdate,
                (
                    fps_controller_spatial_hitter,
                    fps_controller_move,
                    fps_controller_crouch,
                    fps_controller_lean,
                )
                    .chain(),
            );
    }
}

#[derive(Bundle, Default)]
pub struct PlayerControllerBundle {
    pub controller: GoldenController,
    pub keys: GoldenControllerKeys,
    pub mutables: GoldenControllerMutables,
    pub input: GoldenControllerInput,
    pub spatial_hits: GoldenControllerSpatialHits,
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

#[derive(Component)]
pub struct GoldenControllerInput {
    pub jump: bool,
    pub crouch: bool,
    pub pitch: f32,
    pub yaw: f32,
    pub movement: Vec3,
    pub lean: f32, // -1.0 left, +1.0 right
    pub lean_degree_mod: f32,
    pub crouch_degree_mod: f32,
}

impl Default for GoldenControllerInput {
    fn default() -> Self {
        Self {
            jump: false,
            crouch: false,
            pitch: -TAU / 12.0,
            yaw: TAU * 5.0 / 8.0,
            movement: Vec3::ZERO,
            lean: 0.0,
            lean_degree_mod: 0.0,
            crouch_degree_mod: 1.0,
        }
    }
}

#[derive(Component)]
pub struct GoldenController {
    pub radius: f32,

    /// If the distance to the ground is less than this value, the player is considered grounded
    pub grounded_distance: f32,
    pub walk_speed: f32,

    pub forward_speed: f32,
    pub side_speed: f32,
    pub air_speed_cap: f32,

    pub air_acceleration: f32,

    pub acceleration: f32,

    pub crouch_speed: f32,
    /// If the dot product (alignment) of the normal of the surface and the upward vector,
    /// which is a value from [-1, 1], is greater than this value, ground movement is applied
    pub traction_normal_cutoff: f32,

    pub height: f32,

    pub air_damp: f32,

    pub friction: f32,
    pub mass: f32,

    pub enable_input: bool,

    pub jump_force: f32,
    pub lean_max: f32,

    pub air_friction: f32,
    pub lean_side_impulse: f32,
    pub leaning_speed: f32,
}

impl Default for GoldenController {
    fn default() -> Self {
        Self {
            //used for projecting collision to ground, to check if player has traction
            grounded_distance: 0.4,
            //collider height and radius
            radius: 0.4,
            height: 1.0,

            walk_speed: 6.0,
            mass: 80.0,

            //how fast your character enters crouched position
            crouch_speed: 4.0,

            //air friction is actually contact friction to objects while in air, dont change this
            air_friction: 0.0,
            //air damp is actual air friction
            air_damp: 0.3,
            //force to apply when jumping, higher force = higher jumps
            jump_force: 6.0,

            forward_speed: 30.0,
            side_speed: 30.0,
            air_speed_cap: 2.0,

            //how fast you can rotate in the air, higher value allows you to surf like in cs
            air_acceleration: 9.0,

            //max angle degree of leaning, if it is too high there can be clipping bugs when turning fast, very bad
            lean_max: 0.35,
            //how fast you lean
            leaning_speed: 2.0,

            //how fast the player accelerates on the ground, a too low value can break horizontal movement when leaning
            acceleration: 4.0,

            traction_normal_cutoff: 0.6,
            friction: 0.0,

            //how much to move horizontally while leaning
            lean_side_impulse: 65.0,

            enable_input: true,
        }
    }
}
// should probably add normal keybinds for scrolling/lean degree
#[derive(Component)]
pub struct GoldenControllerKeys {
    pub key_forward: KeyCode,
    pub key_back: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_lean_left: KeyCode,
    pub key_lean_right: KeyCode,
    pub key_crouch: KeyCode,
    pub key_jump: KeyCode,
}

impl Default for GoldenControllerKeys {
    fn default() -> Self {
        Self {
            key_forward: KeyCode::KeyW,
            key_back: KeyCode::KeyS,
            key_left: KeyCode::KeyA,
            key_right: KeyCode::KeyD,
            key_lean_left: KeyCode::KeyQ,
            key_lean_right: KeyCode::KeyE,
            key_crouch: KeyCode::ShiftLeft,
            key_jump: KeyCode::Space,
        }
    }
}

#[derive(Component)]
pub struct GoldenControllerMutables {
    pub ground_tick: u8,
    pub pitch: f32,
    pub yaw: f32,
    pub lean_degree: f32,
    pub sensitivity: f32,
    pub crouch_degree: f32,
}
#[derive(Component, Default)]
pub struct GoldenControllerSpatialHits {
    pub top_up: bool,
    pub bottom_down: bool,
    pub bottom_down_distance: f32,
    pub bottom_hit_normal: Vec3,
    pub right_wall_dist: (bool, f32),
    pub left_wall_dist: (bool, f32),
}

impl Default for GoldenControllerMutables {
    fn default() -> Self {
        Self {
            //degrees determine the amount you are currently crouched/leaned, used for variable crouching and leaning
            crouch_degree: 0.0,
            lean_degree: 0.0,
            //how long you have been on the ground, used for some hacky stuff
            ground_tick: 0,
            pitch: 0.0,
            yaw: 0.0,
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
const CALC_EPSILON: f32 = 0.01;

const SLIGHT_SCALE_DOWN: f32 = 0.9;

pub fn fps_controller_move(
    mut query: Query<
        (
            &GoldenControllerInput,
            &GoldenController,
            &GoldenControllerSpatialHits,
            &mut GoldenControllerMutables,
            &mut LinearVelocity,
            &mut ExternalImpulse,
            &mut LinearDamping,
        ),
        With<LogicalPlayer>,
    >,
) {
    let dt = 1.0 / FPS as f32;

    for (
        input,
        controller,
        spatial_hits,
        mut controller_mutables,
        mut velocity,
        mut external_force,
        mut damping,
    ) in query.iter_mut()
    {
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
        let max_speed = (controller.walk_speed
            * (1.0 - controller_mutables.crouch_degree / 2.0)
            * (1.0 - controller_mutables.lean_degree.abs() / 2.0))
            .max(3.0);
        wish_speed = f32::min(wish_speed, max_speed);

        if spatial_hits.bottom_down {
            damping.0 = controller.air_damp * 10.0;
            if !input.jump {
                let add = acceleration(
                    wish_direction,
                    wish_speed,
                    controller.acceleration,
                    velocity.0,
                    dt,
                );
                external_force.apply_impulse(add * controller.mass);
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

                external_force.apply_impulse(add * controller.mass);
            }
            // check if player is on walkable slope
            let has_traction = Vec3::dot(spatial_hits.bottom_hit_normal, Vec3::Y)
                > controller.traction_normal_cutoff;
            if has_traction {
                if !input.jump {
                    let current_height = spatial_hits.bottom_down_distance;
                    let target_height = 0.5;
                    let height_error = target_height - current_height;
                    let freq = 0.8;
                    let damping = 0.8;

                    let omega = 2.0 * std::f32::consts::PI * freq;
                    let k = controller.mass * omega * omega;
                    let c = 2.0 * controller.mass * damping * omega;

                    let vertical_vel = velocity.0.y;
                    let f_spring = k * height_error;
                    let f_damp = -c * vertical_vel;
                    let f_total = f_spring + f_damp + controller.mass * 9.81;

                    external_force.apply_impulse(Vec3::Y * f_total * dt);
                }

                //This fixes bug that pushes player randomly upon landing
                let linear_velocity = velocity.0;
                let normal_force = Vec3::dot(linear_velocity, spatial_hits.bottom_hit_normal)
                    * spatial_hits.bottom_hit_normal;
                velocity.0 -= normal_force;

                if !input.jump && input.movement.length_squared() < 0.1 {
                    damping.0 = controller.air_damp * 30.0;
                }

                if input.jump && velocity.0.y < 1.0 {
                    let jump_force = Vec3 {
                        x: 0.0,
                        y: controller.jump_force,
                        z: 0.0,
                    } * controller.mass;
                    external_force.apply_impulse(jump_force);
                }
            }
            controller_mutables.ground_tick = controller_mutables.ground_tick.saturating_add(1);
        } else {
            controller_mutables.ground_tick = 0;

            damping.0 = controller.air_damp;

            wish_speed = f32::min(wish_speed, controller.air_speed_cap);

            let add = acceleration(
                wish_direction,
                wish_speed,
                controller.air_acceleration,
                velocity.0,
                dt,
            );

            external_force.apply_impulse(add * controller.mass);
        }

        //  Fixes wobbly velocity
        if velocity.0.z.abs() < 0.004 {
            velocity.0.z = 0.0;
        }
        if velocity.0.x.abs() < 0.004 {
            velocity.0.x = 0.0;
        }
    }
}

pub fn fps_controller_spatial_hitter(
    spatial_query_pipeline: Res<SpatialQueryPipeline>,
    mut query: Query<
        (
            Entity,
            &GoldenControllerInput,
            &GoldenController,
            &mut GoldenControllerSpatialHits,
            &Collider,
            &mut Transform,
        ),
        With<LogicalPlayer>,
    >,
) {
    for (entity, input, controller, mut spatial_hits, collider, transform) in query.iter_mut() {
        // Shape cast downwards to find ground
        // Better than a ray cast as it handles when you are near the edge of a surface
        let filter = SpatialQueryFilter::default().with_excluded_entities([entity]);

        let speeds = Vec3::new(controller.side_speed, 0.0, controller.forward_speed);
        let mut move_to_world = Mat3::from_axis_angle(Vec3::Y, input.yaw);
        move_to_world.z_axis *= -1.0; // Forward is -Z
        let mut wish_direction = move_to_world * (input.movement * speeds);
        let wish_speed = wish_direction.length();
        if wish_speed > f32::EPSILON {
            // Avoid division by zero
            wish_direction /= wish_speed; // Effectively normalize, avoid length computation twice
        }
        let foot_shape = Collider::cylinder(controller.radius * 0.95, 0.2);
        let feet_origin = transform.translation - collider_y_offset(&collider)
            + Vec3::new(0.0, controller.grounded_distance, 0.0);
        let bottom_down_hit = spatial_query_pipeline.cast_shape(
            // Consider when the controller is right up against a wall
            // We do not want the shape cast to detect it,
            // so provide a slightly smaller collider in the XZ plane
            &foot_shape,
            feet_origin,
            transform.rotation,
            -Dir3::Y,
            &ShapeCastConfig::from_max_distance(
                controller.grounded_distance, //+ controller.lean_degree.abs() / 20.0 hack to stay grounded while leaning
            ),
            &filter,
        );
        match bottom_down_hit {
            // NEAR GROUND
            Some(hit) => {
                spatial_hits.bottom_down = true;
                spatial_hits.bottom_hit_normal = hit.normal1;
                spatial_hits.bottom_down_distance = hit.distance;
            }

            None => {
                spatial_hits.bottom_down = false;
                spatial_hits.bottom_hit_normal = Vec3::ZERO;
            }
        }
        // the top hit should be at least the stair height so that the player isnt translated inside a roof
        let top_up_hit = spatial_query_pipeline.cast_shape(
            &scaled_collider_laterally(&collider, 0.99),
            transform.translation + Vec3::new(0.0, controller.height, 0.0),
            Quat::IDENTITY,
            Dir3::Y,
            &ShapeCastConfig::from_max_distance(controller.grounded_distance),
            &filter,
        );
        if top_up_hit.is_some() {
            spatial_hits.top_up = true;
        } else {
            spatial_hits.top_up = false;
        }

        let yaw_rotation = Quat::from_euler(EulerRot::YXZ, input.yaw, 0.0, 0.0);
        let right_dir = yaw_rotation * Vec3::X; // world-space right

        let probe_origin = transform.translation + Vec3::new(0.0, 0.1, 0.0);
        let probe_distance = 1.0;
        let side_shape = &scaled_collider_laterally(&collider, SLIGHT_SCALE_DOWN);

        // Right wall check
        let right_hit = spatial_query_pipeline.cast_shape(
            &side_shape,
            probe_origin,
            transform.rotation,
            Dir3::new(right_dir).unwrap(),
            &ShapeCastConfig::from_max_distance(probe_distance),
            &filter,
        );

        match right_hit {
            Some(h) => spatial_hits.right_wall_dist = (true, h.distance),
            None => spatial_hits.right_wall_dist = (false, 1.0),
        }

        // Left wall check
        let left_hit = spatial_query_pipeline.cast_shape(
            &side_shape,
            probe_origin,
            transform.rotation,
            Dir3::new(-right_dir).unwrap(),
            &ShapeCastConfig::from_max_distance(probe_distance),
            &filter,
        );
        match left_hit {
            Some(h) => spatial_hits.left_wall_dist = (true, h.distance),
            None => spatial_hits.left_wall_dist = (false, 1.0),
        }
    }
}

pub fn fps_controller_lean(
    mut query: Query<
        (
            &GoldenControllerInput,
            &GoldenController,
            &GoldenControllerSpatialHits,
            &mut GoldenControllerMutables,
            &mut Transform,
        ),
        With<LogicalPlayer>,
    >,
) {
    let dt = 1.0 / FPS as f32;

    for (input, controller, spatial_hits, mut controller_mutables, mut transform) in
        query.iter_mut()
    {
        /* Leaning */
        let yaw_rotation = Quat::from_euler(EulerRot::YXZ, input.yaw, 0.0, 0.0);
        let right_dir = yaw_rotation * Vec3::X; // world-space right

        let lean_step = controller.leaning_speed * dt;

        // Desired lean from input
        let mut target_lean = input.lean;

        // Block intentional lean into wall
        if spatial_hits.right_wall_dist.0 && (target_lean > 0.0) {
            target_lean = spatial_hits.right_wall_dist.1;
        }
        if spatial_hits.left_wall_dist.0 && (target_lean < 0.0) {
            target_lean = -spatial_hits.left_wall_dist.1;
        }
        controller_mutables.lean_degree = controller_mutables.lean_degree.clamp(
            -spatial_hits.left_wall_dist.1,
            spatial_hits.right_wall_dist.1,
        );
        let old_degree = controller_mutables.lean_degree;

        // Apply lean degree modifier
        target_lean *= 1.0 - input.lean_degree_mod;

        // Smooth toward target with epsilon deadzone
        if (controller_mutables.lean_degree - target_lean).abs() > CALC_EPSILON {
            controller_mutables.lean_degree +=
                lean_step * (target_lean - controller_mutables.lean_degree).signum();
        } else {
            controller_mutables.lean_degree = target_lean;
        }

        controller_mutables.lean_degree = controller_mutables.lean_degree.clamp(
            -spatial_hits.left_wall_dist.1,
            spatial_hits.right_wall_dist.1,
        );

        let degree_change = controller_mutables.lean_degree - old_degree;

        // Shift collider sideways to simulate body lean (peeking)
        transform.translation += right_dir * controller.lean_side_impulse * degree_change * dt;

        // Rotate to show visual lean
        let lean_amount = controller_mutables.lean_degree * controller.lean_max;
        let lean_rotation = Quat::from_axis_angle(Vec3::Z, -lean_amount);
        transform.rotation = (yaw_rotation * lean_rotation).normalize();
    }
}
pub fn fps_controller_crouch(
    mut query: Query<
        (
            &GoldenControllerInput,
            &GoldenController,
            &GoldenControllerSpatialHits,
            &mut GoldenControllerMutables,
            &mut Collider,
        ),
        With<LogicalPlayer>,
    >,
) {
    let dt = 1.0 / FPS as f32;

    for (input, controller, spatial_hits, mut controller_mutables, mut collider) in query.iter_mut()
    {
        /* Crouching */

        // Target crouch state: 1 = crouch, 0 = stand
        let target_crouch = if input.crouch {
            input.crouch_degree_mod
        } else {
            0.0
        };

        // Smoothly move actual crouch_degree toward target
        if (controller_mutables.crouch_degree - target_crouch).abs() > CALC_EPSILON {
            if controller_mutables.crouch_degree < target_crouch {
                controller_mutables.crouch_degree += controller.crouch_speed * dt;
            } else if controller_mutables.crouch_degree > target_crouch {
                // Only allow standing up if there's no ceiling
                if !spatial_hits.top_up {
                    controller_mutables.crouch_degree -= controller.crouch_speed * dt;
                }
            }
        } else {
            // Snap to target when within epsilon to avoid jitter
            controller_mutables.crouch_degree = target_crouch;
        }

        // Clamp for safety
        controller_mutables.crouch_degree = controller_mutables.crouch_degree.clamp(0.0, 1.0);

        // Update collider height

        let current_height =
            (controller.height / 2.0) / (4.0 * controller_mutables.crouch_degree + 1.0);
        collider.set_shape(SharedShape::capsule_y(current_height, controller.radius));
    }
}

/*
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 */

pub fn fps_controller_input(
    key_input: Res<ButtonInput<KeyCode>>,
    mut mouse_events: EventReader<MouseMotion>,
    mut query: Query<(
        &GoldenControllerKeys,
        &GoldenControllerMutables,
        &mut GoldenControllerInput,
    )>,
) {
    for (controller, controller_mutables, mut input) in query.iter_mut() {
        let mut mouse_delta = Vec2::ZERO;
        for mouse_event in mouse_events.read() {
            mouse_delta += mouse_event.delta;
        }
        mouse_delta *= controller_mutables.sensitivity;

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
    mut query: Query<&mut GoldenControllerInput>,
) {
    let mut mod_shift = 0.0;

    for ev in evr_scroll.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                if ev.y.abs() > 0.1 {
                    mod_shift += ev.y.signum() * 0.1;
                }
                if ev.x.abs() > 0.1 {
                    mod_shift += ev.x.signum() * 0.1;
                }
            }
            MouseScrollUnit::Pixel => {
                if ev.y.abs() > 0.1 {
                    mod_shift += ev.y.signum() * 0.1;
                }
                if ev.x.abs() > 0.1 {
                    mod_shift += ev.x.signum() * 0.1;
                }
            }
        }
    }
    mod_shift = mod_shift.clamp(-1.0, 1.0);

    for mut input in query.iter_mut() {
        if input.lean.abs() > 0.1 {
            input.lean_degree_mod += mod_shift;
            input.lean_degree_mod = input.lean_degree_mod.clamp(0.0, 1.0);
        } else if input.crouch {
            input.crouch_degree_mod -= mod_shift;
            input.crouch_degree_mod = input.crouch_degree_mod.clamp(0.0, 1.0);
        }
    }
}

pub fn fps_controller_look(
    mut query: Query<(&mut GoldenControllerMutables, &GoldenControllerInput)>,
) {
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
        } else if let Some(capsule) = collider.shape().as_capsule() {
            capsule.half_height() + capsule.radius
        } else {
            panic!("Controller must use a cylinder or capsule collider")
        }
}

/// Return a collider that is scaled laterally (XZ plane) but not vertically (Y axis).
fn scaled_collider_laterally(collider: &Collider, scale: f32) -> Collider {
    if let Some(cylinder) = collider.shape().as_cylinder() {
        let new_cylinder = Collider::cylinder(cylinder.radius * scale, cylinder.half_height * 2.0);
        new_cylinder
    } else if let Some(capsule) = collider.shape().as_capsule() {
        let new_capsule = Collider::capsule(capsule.radius * scale, capsule.segment.length());
        new_capsule
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
        (
            &Transform,
            &Collider,
            &GoldenControllerMutables,
            &CameraConfig,
        ),
        (With<LogicalPlayer>, Without<RenderPlayer>),
    >,
) {
    for (mut render_transform, render_player) in render_query.iter_mut() {
        if let Ok((logical_transform, collider, controller_mutables, camera_config)) =
            logical_query.get(render_player.logical_entity)
        {
            let collider_offset = collider_y_offset(collider);
            let camera_offset = Vec3::Y * camera_config.height_offset;
            render_transform.translation =
                logical_transform.translation + collider_offset + camera_offset;
            let pitch_quat = Quat::from_euler(EulerRot::YXZ, 0.0, controller_mutables.pitch, 0.0);
            render_transform.rotation = logical_transform.rotation.mul_quat(pitch_quat);
        }
    }
}
