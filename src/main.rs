mod control;
mod skybox;

use std::f32::consts::PI;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, pbr::NotShadowCaster, prelude::*, render::render_resource::Face, window::{PresentMode, PrimaryWindow}
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use control::ControllerPlugin;
use skybox::SkyboxPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            bevy_framepace::FramepacePlugin,
            WorldInspectorPlugin::new(),
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin,
            SkyboxPlugin,
        ))
        .add_plugins(ControllerPlugin)
        .add_systems(Startup, (startup_spawner, disable_vsync))
        .add_systems(
            Update,
            watch_load_events,
        )
        .add_systems(
            Update,
            sub_controls.run_if(in_state(control::ControlState::Unfocused)),
        )
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0,
        })
        .register_type::<SubControls>()
        .run();
}

#[derive(Debug, Resource)]
struct PoolMesh(Handle<Gltf>);

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

pub fn startup_spawner(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: ResMut<AssetServer>,
) {
    let pool_mesh: Handle<Gltf> = asset_server.load("models/pool.glb");
    commands.insert_resource(PoolMesh(pool_mesh));

    let mut water_material = StandardMaterial {
        perceptual_roughness: 0.0,
        specular_transmission: 1.0,
        thickness: 0.2,
        ior: 1.33,
        base_color: Color::srgb(0.2, 0.5, 0.7),
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: None,
        ..default()
    };

    // Camera
    commands.spawn(Camera3dBundle::default());

    // Cuboid
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::from_size(Vec3::new(10., 5., 14.) - Vec3::splat(1e-3))),
            // mesh: meshes.add(Cuboid::new(5., 1., 4.)),
            material: materials.add(water_material.clone()),
            transform: Transform::from_translation(Vec3::new(0., 2.5, 0.)),
            ..default()
        },
        NotShadowCaster,
        Name::new("Water"),
    ));

    // Sub
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(1., 0.5, 1.)),
            // material: materials.add(StandardMaterial {
            //     base_color: Color::srgb(0.4, 0.2, 0.2),
            //     ..default()
            // }),
            material: materials.add(water_material),
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..default()
        },
        Name::new("Sub"),
        SubControls::new(0.05),
    ));

    // Light
    commands.spawn((
        DirectionalLightBundle {
            transform: Transform::from_rotation(Quat::from_euler(
                EulerRot::ZYX,
                1.0,
                1.0,
                -PI / 4.,
            )),
            directional_light: DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            ..default()
        },
        Name::new("Sun"),
    ));
}

fn disable_vsync(mut window: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = window.get_single_mut().unwrap();
    window.present_mode = PresentMode::AutoNoVsync;
}

fn watch_load_events(
    mut commands: Commands,
    mut events: EventReader<AssetEvent<Gltf>>,
    gltf_assets: Res<Assets<Gltf>>,
    pool: Res<PoolMesh>,
) {
    let pool_id = pool.0.id();
    for event in events.read() {
        match event {
            AssetEvent::LoadedWithDependencies { id } if *id == pool_id => {
                commands.spawn((
                    SceneBundle {
                        scene: gltf_assets
                            .get(pool_id)
                            .unwrap()
                            .default_scene
                            .clone()
                            .unwrap(),
                        ..default()
                    },
                    Name::new("Pool"),
                ));
            }
            _ => {}
        }
    }
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
