mod control;
mod frustum_gizmo;
mod hal;
mod skybox;
mod utils;

use std::f32::consts::PI;

use avian3d::{
    PhysicsPlugins,
    prelude::{
        AngularDamping, AngularInertia, AngularInertiaTensor, CenterOfMass, Collider,
        ComputedCenterOfMass, ComputedMass, ExternalForce, Gravity, LinearDamping, Mass,
        PhysicsDebugPlugin, RigidBody,
    },
};
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    pbr::NotShadowCaster,
    prelude::*,
    render::render_resource::TextureUsages,
    window::{PresentMode, PrimaryWindow},
};
use bevy_egui::{EguiContextSettings, EguiPlugin};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use control::ControllerPlugin;
use frustum_gizmo::{FrustumGizmoPlugin, ShowFrustumGizmo};
use hal::{BotCamImage, HalPlugin, ImageExportSource, ZedImage};
use rand::{SeedableRng as _, rngs::StdRng};
use skybox::SkyboxPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            bevy_framepace::FramepacePlugin,
            EguiPlugin::default(),
            WorldInspectorPlugin::new(),
            LogDiagnosticsPlugin::default(),
            // FrameTimeDiagnosticsPlugin::default(),
            SkyboxPlugin,
            PhysicsPlugins::default(),
            PhysicsDebugPlugin::default(),
            FrustumGizmoPlugin::default(),
            HalPlugin::default(),
        ))
        .add_plugins(ControllerPlugin)
        .add_systems(Startup, (startup_spawner, disable_vsync))
        .add_sub_state::<TeleopState>()
        .add_systems(
            Update,
            set_teleop_state.run_if(in_state(control::ControlState::Unfocused)),
        )
        .add_systems(Update, sub_controls.run_if(in_state(TeleopState::Teleop)))
        .add_systems(Update, |mut gizmos: Gizmos| {
            gizmos.axes(Transform::default(), 1.)
        })
        .add_systems(
            Update,
            |thrusters: Query<&GlobalTransform, With<ThrusterOf>>, mut gizmos: Gizmos| {
                for transform in thrusters {
                    gizmos.axes(transform.compute_transform(), 0.05)
                }
            },
        )
        .add_systems(
            FixedUpdate,
            (
                buoyancy,
                update_thruster_speeds,
                debug_thruster_speeds.after(update_thruster_speeds),
                thruster_physics.after(update_thruster_speeds),
            )
                .in_set(VehiclePhysicsSet),
        )
        // .add_systems(FixedUpdate, save_image_periodically)
        .init_resource::<BuoyancySamples>()
        .register_type::<VehiclePhysicsSet>()
        .register_type::<ViewCamera>()
        .register_type::<ZedCamera>()
        .register_type::<SubControls>()
        .register_type::<WaterCollider>()
        .register_type::<SubBuoyancy>()
        .register_type::<BuoyancySamples>()
        .register_type::<ThrusterOf>()
        .register_type::<Thrusters>()
        .register_type::<ThrusterTarget>()
        .register_type::<ThrusterState>()
        .register_type::<ThrusterParams>()
        .run();
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect, Hash, SystemSet)]
struct VehiclePhysicsSet;

#[derive(Debug, Component, Reflect)]
#[reflect(Component, Debug)]
struct ViewCamera;

#[derive(Debug, Component, Reflect)]
#[reflect(Component, Debug)]
struct ZedCamera;

#[derive(Debug, Component, Reflect)]
#[reflect(Component, Debug)]
struct BottomCamera;

#[derive(Debug, Component, Reflect)]
#[reflect(Component, Debug)]
struct SubControls {
    scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(control::ControlState = control::ControlState::Unfocused)]
pub enum TeleopState {
    #[default]
    NoTeleop,
    Teleop,
}

impl SubControls {
    fn new(scale: f32) -> Self {
        Self { scale }
    }
}

#[derive(Debug, Component, Reflect, Deref, Clone, Copy)]
#[reflect(Component, Debug)]
struct WaterCollider(Cuboid);

#[derive(Debug, Component, Reflect, Deref, Clone, Copy)]
#[reflect(Component)]
struct Buoyant<S: ShapeSample> {
    #[deref]
    shape: S,
    buoyancy_factor: f32,
}

impl<S: ShapeSample> Buoyant<S> {
    pub fn new(shape: S, buoyancy_factor: f32) -> Self {
        Self {
            shape,
            buoyancy_factor,
        }
    }
}

type SubBuoyancy = Buoyant<Cuboid>;

pub fn spawn_zed(
    commands: &mut Commands,
    sub: Entity,
    images: &mut Assets<Image>,
    export_sources: &mut Assets<ImageExportSource>,
) {
    let mut image = Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: 640,
            height: 480,
            ..default()
        },
        bevy::render::render_resource::TextureDimension::D2,
        &[0, 255, 0, 255],
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    image.texture_descriptor.label = Some("Zed cam target");
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC;
    let target = images.add(image);
    commands.insert_resource(ZedImage(export_sources.add(target.clone())));
    commands.spawn((
        Camera3d::default(),
        Camera {
            target: bevy::render::camera::RenderTarget::Image(target.into()),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.08, 0.0, 0.03)).with_rotation(
            Quat::from_axis_angle(Vec3::Y, -core::f32::consts::FRAC_PI_2),
        ),
        ShowFrustumGizmo::default(),
        ZedCamera,
        ChildOf(sub),
        Name::new("Zed cam"),
    ));
}

pub fn startup_spawner(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut export_sources: ResMut<Assets<ImageExportSource>>,
) {
    let water_material = materials.add(StandardMaterial {
        perceptual_roughness: 0.0,
        specular_transmission: 1.0,
        // thickness: 0.2,
        // ior: 1.33,
        base_color: Color::srgb(0.2, 0.5, 0.7),
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: None,
        ..default()
    });

    // Camera
    commands.spawn((
        Camera3d::default(),
        ViewCamera,
        EguiContextSettings::default(),
        Name::new("Main cam"),
    ));

    // Pool
    let outer_half_size = Vec3::new(12., 6., 12.);
    let inner_half_size = Vec3::new(10., 5., 10.);
    let pool = spawn_pool(
        &mut *meshes,
        &mut *materials,
        &mut commands,
        outer_half_size,
        inner_half_size,
    );
    let water_cuboid = Cuboid {
        half_size: inner_half_size - Vec3::splat(1e-3),
    };
    commands.spawn((
        ChildOf(pool),
        Mesh3d(meshes.add(water_cuboid)),
        MeshMaterial3d(water_material.clone()),
        NotShadowCaster,
        WaterCollider(water_cuboid),
        Name::new("Water"),
    ));

    // Sub
    let sub = spawn_sub(&mut *meshes, &mut *materials, &mut commands);
    spawn_zed(&mut commands, sub, &mut *images, &mut *export_sources);

    // Light
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 1.0, 1.0, -PI / 4.)),
        Name::new("Sun"),
    ));
}

fn spawn_pool(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    commands: &mut Commands,
    o: Vec3,
    i: Vec3,
) -> Entity {
    let a1 = Vec3::new(o[0], -o[1], -i[2]);
    let a2 = Vec3::new(-i[0], i[1], -o[2]);
    let b1 = Vec3::new(o[0], -o[1], -i[2]);
    let b2 = Vec3::new(i[0], i[1], o[2]);
    let c1 = Vec3::new(-o[0], -o[1], i[2]);
    let c2 = Vec3::new(i[0], i[1], o[2]);
    let d1 = Vec3::new(-o[0], -o[1], i[2]);
    let d2 = Vec3::new(-i[0], i[1], -o[2]);
    let f1 = -i;
    let f2 = Vec3::new(i[0], -o[1], i[2]);
    let material = MeshMaterial3d(materials.add(StandardMaterial { ..default() }));
    let mut wall = |x0, x1, name| {
        let cuboid = Cuboid::from_corners(x0, x1);
        let translation = Transform::from_translation(Vec3::midpoint(x0, x1));
        (
            Mesh3d(meshes.add(cuboid)),
            material.clone(),
            translation,
            Collider::from(cuboid),
            RigidBody::Static,
            NotShadowCaster,
            Name::new(name),
        )
    };
    commands
        .spawn((
            Name::new("Pool"),
            Transform::default(),
            Visibility::default(),
            children![
                wall(a1, a2, "Wall0"),
                wall(b1, b2, "Wall1"),
                wall(c1, c2, "Wall2"),
                wall(d1, d2, "Wall3"),
                wall(f1, f2, "Floor"),
            ],
        ))
        .id()
}

const SUB_SIZE: Vec3 = Vec3::new(0.35, 0.15, 0.35);
fn spawn_sub(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    commands: &mut Commands,
) -> Entity {
    let sub_material = materials.add(StandardMaterial {
        base_color: Srgba::GREEN.into(),
        ..default()
    });
    let sub_cuboid = Cuboid::from_size(SUB_SIZE);
    let com = Vec3::new(0., -sub_cuboid.half_size.x / 3., 0.);
    let sub_entity = commands
        .spawn((
            Mesh3d(meshes.add(sub_cuboid)),
            MeshMaterial3d(sub_material),
            Transform::from_translation(Vec3::new(0., 2., 0.)).with_scale(Vec3::splat(3.)),
            Name::new("Sub"),
            SubControls::new(1.0),
            Collider::from(sub_cuboid),
            RigidBody::Dynamic,
            SubBuoyancy::new(sub_cuboid, 1.01),
            // TODO: Make this vary based on % underwater
            (LinearDamping(0.5), AngularDamping(1.0)),
            //
            ExternalForce::ZERO.with_persistence(false),
            CenterOfMass(com),
            Mass(20.),
            // TODO: What might the tensor's value be?
            AngularInertia::from(AngularInertiaTensor::default() / 0.05),
            Thrusters::default(),
        ))
        .id();

    let thruster_material = materials.add(StandardMaterial {
        base_color: Srgba::BLACK.into(),
        ..default()
    });
    let thruster_mesh = Extrusion::new(Annulus::new(0.05, 0.06), 0.11)
        .mesh()
        .build()
        .rotated_by(Quat::from_axis_angle(Vec3::X, core::f32::consts::FRAC_PI_2));
    let thruster_mesh = meshes.add(thruster_mesh);
    let base_translation = com * 0.4;
    let base_rotation = Quat::from_axis_angle(Vec3::Z, -core::f32::consts::FRAC_PI_2);
    for thruster in THRUSTERS {
        commands.spawn((
            ChildOf(sub_entity),
            ThrusterOf {
                sub: sub_entity,
                id: thruster.id,
            },
            ThrusterTarget::default(),
            Transform::from_translation(base_translation + thruster.translation())
                .with_rotation(thruster.rotation() * base_rotation),
            Mesh3d(thruster_mesh.clone()),
            MeshMaterial3d(thruster_material.clone()),
            Mass(0.4),
            Collider::cylinder(0.06, 0.11),
        ));
    }

    sub_entity
}

fn disable_vsync(mut window: Query<&mut Window, With<PrimaryWindow>>) -> Result {
    let mut window = window.single_mut()?;
    window.present_mode = PresentMode::AutoNoVsync;
    Ok(())
}

fn set_teleop_state(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    state: Res<State<TeleopState>>,
    mut commands: Commands,
) {
    if keyboard_input.just_pressed(KeyCode::KeyT) {
        commands.insert_resource(NextState::Pending(match **state {
            TeleopState::NoTeleop => TeleopState::Teleop,
            TeleopState::Teleop => TeleopState::NoTeleop,
        }));
    }
}

fn sub_controls(
    mut thrusters: Query<(&ThrusterOf, &mut ThrusterTarget)>,
    subs: Query<(Entity, &SubControls)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) -> Result {
    let (sub, controls) = subs.single()?;
    let mut linear = Vec3::ZERO;
    let mut yaw = 0.0;
    if keyboard_input.pressed(KeyCode::KeyW) {
        linear += Vec3::X;
    };
    if keyboard_input.pressed(KeyCode::KeyS) {
        linear += Vec3::NEG_X;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        linear += Vec3::NEG_Z;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        linear += Vec3::Z;
    }
    if keyboard_input.pressed(KeyCode::Space) {
        linear += Vec3::Y;
    }
    if keyboard_input.pressed(KeyCode::ShiftLeft) {
        linear += Vec3::NEG_Y;
    }
    if keyboard_input.pressed(KeyCode::KeyQ) {
        yaw -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyE) {
        yaw += 1.0;
    }
    let mut max = 0.0_f32;
    for (info, mut target) in &mut thrusters {
        if info.sub != sub {
            continue;
        }
        target.target_speed = match info.id {
            0 => linear.x - linear.z - yaw,
            1 => linear.x + linear.z + yaw,
            2 => linear.x + linear.z - yaw,
            3 => linear.x - linear.z + yaw,
            4..8 => -linear.y,
            _ => {
                panic!("Unexpected thruster id {} for sub {sub}", info.id)
            }
        };
        max = max.max(target.target_speed.abs());
    }
    if max < 1e-5 {
        return Ok(());
    }
    let scale = controls.scale / max;
    for (info, mut target) in thrusters {
        if info.sub != sub {
            continue;
        }
        target.target_speed *= scale;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Resource, Reflect, PartialEq, Eq)]
#[reflect(Resource, PartialEq, Debug)]
struct BuoyancySamples {
    pub count: u32,
}

impl BuoyancySamples {
    pub fn new(count: u32) -> Self {
        Self { count }
    }
}

impl Default for BuoyancySamples {
    fn default() -> Self {
        Self::new(100)
    }
}

fn add_forces(a: &ExternalForce, b: &ExternalForce, persistence: bool) -> ExternalForce {
    let force = a.force() + b.force();
    let torque = a.torque() + b.torque();
    (force, persistence, torque).into()
}

fn buoyancy(
    gravity: Res<Gravity>,
    samples: Res<BuoyancySamples>,
    subs: Query<
        (
            &GlobalTransform,
            &SubBuoyancy,
            &ComputedMass,
            &ComputedCenterOfMass,
            Entity,
        ),
        With<ExternalForce>,
    >,
    water: Query<(&GlobalTransform, &WaterCollider)>,
    mut gizmos: Gizmos,
    mut commands: Commands,
    mut rng: Local<Option<StdRng>>,
) -> Result {
    // TODO: Is this a sufficiently deterministic rng?
    let mut rng = rng.get_or_insert_with(|| StdRng::from_seed([0; 32]));
    let (water_transform, water_cuboid) = water.single()?;
    let water_inverse = water_transform.affine().inverse();
    for (transform, buoyancy, mass, com, entity) in subs {
        let mut force = ExternalForce::default();
        let gravity_force = gravity.0 * mass.value();
        let global_com = transform.transform_point(com.0);

        let force_per_sample = -gravity_force / (samples.count as f32) * buoyancy.buoyancy_factor;
        for _ in 0..samples.count {
            let local_sample = buoyancy.sample_interior(&mut rng);
            let global_sample = transform.transform_point(local_sample);
            let water_local = water_inverse.transform_point(global_sample);
            let underwater = water_cuboid.closest_point(water_local) == water_local;
            if underwater {
                force.apply_force_at_point(force_per_sample, global_sample, global_com);
                // gizmos.arrow(
                //     global_sample,
                //     global_sample + force_per_sample * 0.1,
                //     Srgba::GREEN,
                // );
            } else {
                // gizmos.arrow(
                //     global_sample,
                //     global_sample + force_per_sample * 0.1,
                //     Srgba::RED,
                // );
            }
        }
        commands
            .entity(entity)
            .entry::<ExternalForce>()
            .or_default()
            .and_modify(move |mut ef| {
                *ef = add_forces(&*ef, &force, false);
            });

        gizmos.arrow(global_com, transform.translation(), Srgba::BLUE);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, PartialEq, Debug)]
#[relationship(relationship_target = Thrusters)]
struct ThrusterOf {
    #[relationship]
    sub: Entity,
    id: u8,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect, Deref)]
#[reflect(Component, PartialEq, Debug)]
#[relationship_target(relationship = ThrusterOf)]
struct Thrusters(Vec<Entity>);

#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, PartialEq, Debug)]
#[require(ThrusterState, ThrusterParams)]
struct ThrusterTarget {
    target_speed: f32,
}

#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, PartialEq, Debug)]
struct ThrusterState {
    speed: f32,
}

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, PartialEq, Debug)]
struct ThrusterParams {
    max_force: f32,
    ease_factor: f32,
}

impl Default for ThrusterParams {
    fn default() -> Self {
        Self {
            max_force: 4.0,   // Approx max reverse force for a T200 thruster at 14.8 V
            ease_factor: 0.2, // Selected by hand
        }
    }
}

fn update_thruster_speeds(
    thrusters: Query<(&ThrusterTarget, &mut ThrusterState, &ThrusterParams)>,
) {
    for (target, mut state, params) in thrusters {
        let diff = target.target_speed - state.speed;
        // TODO: Make this time independent
        // Current solution: run this system in FixedUpdate
        state.speed += diff * params.ease_factor;
    }
}

fn debug_thruster_speeds(
    thrusters: Query<(&GlobalTransform, &ThrusterTarget, &ThrusterState)>,
    mut gizmos: Gizmos,
) {
    for (transform, target, state) in thrusters {
        let start = transform.translation();
        let target_arrow = target.target_speed * transform.up();
        gizmos.arrow(start, start + target_arrow, Srgba::RED);
        let state_arrow = state.speed * transform.up();
        gizmos.arrow(start, start + state_arrow, Srgba::GREEN);
    }
}

fn thruster_physics(
    thrusters: Query<(&GlobalTransform, &ThrusterState, &ThrusterParams)>,
    subs: Query<(&GlobalTransform, &Thrusters, Entity, &ComputedCenterOfMass)>,
    mut commands: Commands,
) -> Result {
    for (sub_transform, sub_thrusters, sub_entity, com) in subs {
        let mut force = ExternalForce::default();
        let sub_com = sub_transform.transform_point(com.0);
        for &thruster in &**sub_thrusters {
            let (thruster_transform, state, params) = thrusters.get(thruster)?;
            let thruster_center = thruster_transform.translation();
            let thruster_force = thruster_transform.up() * state.speed * params.max_force;
            force.apply_force_at_point(thruster_force, thruster_center, sub_com);
        }
        commands
            .entity(sub_entity)
            .entry::<ExternalForce>()
            .or_default()
            .and_modify(move |mut ef| {
                *ef = add_forces(&*ef, &force, false);
            });
    }
    Ok(())
}

struct ThrusterDescriptor {
    id: u8,
    yaw: f32,
    pitch: f32,
    roll: f32,
    surge: f32,
    sway: f32,
    heave: f32,
}

impl ThrusterDescriptor {
    fn translation(&self) -> Vec3 {
        Vec3::new(self.surge, -self.heave, self.sway)
    }

    fn rotation(&self) -> Quat {
        Quat::from_euler(
            EulerRot::YZX,
            -self.yaw.to_radians(),
            -self.pitch.to_radians(),
            self.roll.to_radians(),
        )
    }
}

const CORNER_THRUSTER_SURGE: f32 = 0.2921;
const CENTER_THRUSTER_SURGE: f32 = 0.127;
const THRUSTER_SWAY: f32 = 0.267;
const THRUSTERS: &[ThrusterDescriptor] = &[
    ThrusterDescriptor {
        id: 0,
        yaw: -45., // yaw of the motor axis, we use z down coordinate frame
        pitch: 0., // pitch of the motor axis
        roll: 0.,  // roll of the motor axis
        surge: CORNER_THRUSTER_SURGE, // x position of motor relative to robot origin
        sway: THRUSTER_SWAY, // y position of motor relative to robot origin
        heave: 0., // z position of motor relative to robot origin
    },
    ThrusterDescriptor {
        id: 1,
        yaw: 45.,
        pitch: 0.,
        roll: 0.,
        surge: CORNER_THRUSTER_SURGE,
        sway: -THRUSTER_SWAY,
        heave: 0.,
    },
    ThrusterDescriptor {
        id: 2,
        yaw: 45.,
        pitch: 0.,
        roll: 0.,
        surge: -CORNER_THRUSTER_SURGE,
        sway: THRUSTER_SWAY,
        heave: 0.,
    },
    ThrusterDescriptor {
        id: 3,
        yaw: -45.,
        pitch: 0.,
        roll: 0.,
        surge: -CORNER_THRUSTER_SURGE,
        sway: -THRUSTER_SWAY,
        heave: 0.,
    },
    ThrusterDescriptor {
        id: 4,
        yaw: 0.,
        pitch: 90.,
        roll: 0.,
        surge: CENTER_THRUSTER_SURGE,
        sway: THRUSTER_SWAY,
        heave: 0.,
    },
    ThrusterDescriptor {
        id: 5,
        yaw: 0.,
        pitch: 90.,
        roll: 0.,
        surge: CENTER_THRUSTER_SURGE,
        sway: -THRUSTER_SWAY,
        heave: 0.,
    },
    ThrusterDescriptor {
        id: 6,
        yaw: 0.,
        pitch: 90.,
        roll: 0.,
        surge: -CENTER_THRUSTER_SURGE,
        sway: THRUSTER_SWAY,
        heave: 0.,
    },
    ThrusterDescriptor {
        id: 7,
        yaw: 0.,
        pitch: 90.,
        roll: 0.,
        surge: -CENTER_THRUSTER_SURGE,
        sway: -THRUSTER_SWAY,
        heave: 0.,
    },
];
