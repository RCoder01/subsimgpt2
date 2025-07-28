mod control;
mod skybox;

use std::f32::consts::PI;

use avian3d::{
    prelude::{
        AngularDamping, CenterOfMass, Collider, ColliderMassProperties, ComputedCenterOfMass,
        ExternalForce, Gravity, LinearDamping, RigidBody,
    },
    PhysicsPlugins,
};
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    pbr::NotShadowCaster,
    prelude::*,
    window::{PresentMode, PrimaryWindow},
};
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use control::ControllerPlugin;
use rand::{rngs::StdRng, SeedableRng as _};
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
        ))
        .add_plugins(ControllerPlugin)
        .add_systems(Startup, (startup_spawner, disable_vsync))
        .add_systems(
            Update,
            sub_controls.run_if(in_state(control::ControlState::Unfocused)),
        )
        .add_systems(FixedUpdate, (buoyancy,).in_set(VehiclePhysicsSet))
        .init_resource::<BuoyancySamples>()
        .register_type::<BuoyancySamples>()
        .register_type::<SubControls>()
        .register_type::<SubBuoyancy>()
        .run();
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect, Hash, SystemSet)]
struct VehiclePhysicsSet;

#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
struct SubControls {
    scale: f32,
}

impl SubControls {
    fn new(scale: f32) -> Self {
        Self { scale }
    }
}

#[derive(Debug, Component, Reflect, Deref, Clone, Copy)]
#[reflect(Component)]
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

pub fn startup_spawner(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let color_material = materials.add(StandardMaterial {
        base_color: Srgba::GREEN.into(),
        ..default()
    });
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
    commands.spawn(Camera3d::default());

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
    let sub_cuboid = Cuboid::new(1., 0.5, 1.);
    commands.spawn((
        Mesh3d(meshes.add(sub_cuboid)),
        MeshMaterial3d(color_material.clone()),
        Transform::from_translation(Vec3::new(0., 2., 0.)),
        Name::new("Sub"),
        SubControls::new(0.05),
        Collider::from(sub_cuboid),
        RigidBody::Dynamic,
        // SubBuoyancy::new(sub_cuboid, 0.0075),
        SubBuoyancy::new(sub_cuboid, 1.05),
        // TODO: Make this vary based on how underwater we are
        LinearDamping(1.0),
        AngularDamping(1.0),
        //
        ExternalForce::ZERO.with_persistence(false),
        CenterOfMass(Vec3::new(0., -0.12, 0.)),
    ));

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

fn disable_vsync(mut window: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = window.single_mut().unwrap();
    window.present_mode = PresentMode::AutoNoVsync;
}

fn sub_controls(
    mut subs: Query<(&mut Transform, &SubControls)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let mut translation = Vec3::ZERO;
    if keyboard_input.pressed(KeyCode::KeyW) {
        translation += Vec3::NEG_Z;
    };
    if keyboard_input.pressed(KeyCode::KeyS) {
        translation += Vec3::Z;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        translation += Vec3::NEG_X;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        translation += Vec3::X;
    }
    if keyboard_input.pressed(KeyCode::KeyE) {
        translation += Vec3::Y;
    }
    if keyboard_input.pressed(KeyCode::KeyQ) {
        translation += Vec3::NEG_Y;
    }
    for (mut transform, scale) in subs.iter_mut() {
        let global = transform.rotation.mul_vec3(translation);
        transform.translation += global * scale.scale;
    }
}

#[derive(Debug, Clone, Copy, Resource, Reflect)]
#[reflect(Resource)]
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
            &ColliderMassProperties,
            &ComputedCenterOfMass,
            Entity,
        ),
        With<ExternalForce>,
    >,
    water: Query<(&GlobalTransform, &WaterCollider)>,
    mut gizmos: Gizmos,
    mut commands: Commands,
    mut rng: Local<Option<StdRng>>,
) {
    // TODO: Is this a sufficiently deterministic rng?
    let mut rng = rng.get_or_insert_with(|| StdRng::from_seed([0; 32]));
    let (water_transform, water_cuboid) = water.single().unwrap();
    let water_inverse = water_transform.affine().inverse();
    for (transform, buoyancy, collider, com, entity) in subs {
        let mut force = ExternalForce::default();
        let gravity_force = gravity.0 * collider.mass;
        let global_com = transform.transform_point(com.0);

        let force_per_sample = -gravity_force / (samples.count as f32) * buoyancy.buoyancy_factor;
        for _ in 0..samples.count {
            let local_sample = buoyancy.sample_interior(&mut rng);
            let global_sample = transform.transform_point(local_sample);
            let water_local = water_inverse.transform_point(global_sample);
            let underwater = water_cuboid.closest_point(water_local) == water_local;
            if underwater {
                force.apply_force_at_point(force_per_sample, global_sample, global_com);
                gizmos.arrow(
                    global_sample,
                    global_sample + force_per_sample,
                    Srgba::GREEN,
                );
            } else {
                gizmos.arrow(global_sample, global_sample + force_per_sample, Srgba::RED);
            }
        }
        commands
            .entity(entity)
            .entry::<ExternalForce>()
            .or_default()
            .and_modify(move |mut ef| {
                *ef = add_forces(&*ef, &force, false);
            });

        gizmos.arrow(transform.translation(), global_com, Srgba::BLUE);
    }
}
