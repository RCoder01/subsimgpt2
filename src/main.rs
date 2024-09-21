mod control;

use std::f32::consts::PI;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, PrimaryWindow},
};
use control::ControllerPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            bevy_framepace::FramepacePlugin,
            // WorldInspectorPlugin::new(),
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin,
        ))
        .add_plugins(ControllerPlugin)
        .add_systems(Startup, (startup_spawner, disable_vsync))
        .add_systems(Update, (fade_transparency.run_if(|| false), watch_load_events))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0,
        })
        .run();
}

#[derive(Debug, Resource)]
struct PoolMesh(Handle<Gltf>);

fn startup_spawner(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: ResMut<AssetServer>,
) {
    commands.spawn(Camera3dBundle::default());
    let pool_mesh: Handle<Gltf> = asset_server.load("models/pool.glb");
    commands.insert_resource(PoolMesh(pool_mesh));

    // Cuboid
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(9.99, 1.99, 13.99)),
        material: materials.add(StandardMaterial {
            perceptual_roughness: 0.0,
            specular_transmission: 1.0,
            thickness: 0.2,
            ior: 1.33,
            base_color: Color::srgb(0.2, 0.5, 0.7),
            alpha_mode: AlphaMode::Mask(0.5),
            ..default()
        }),
        transform: Transform::from_translation(Vec3::new(0., 1., 0.)),
        ..default()
    });

    // Light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 1.0, 1.0, -PI / 4.)),
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });
}

fn disable_vsync(mut window: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = window.get_single_mut().unwrap();
    window.present_mode = PresentMode::AutoNoVsync;
}

fn fade_transparency(time: Res<Time>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let alpha = (time.elapsed_seconds().sin() / 2.0) + 0.5;
    for (_, material) in materials.iter_mut() {
        material.base_color.set_alpha(alpha);
    }
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
                // commands.spawn(SceneBundle {
                //     scene: gltf_assets
                //         .get(pool_id)
                //         .unwrap()
                //         .default_scene
                //         .clone()
                //         .unwrap(),
                //     ..default()
                // });
            }
            _ => {}
        }
    }
}
