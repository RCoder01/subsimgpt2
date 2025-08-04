mod pool;
mod sub;

use bevy::{pbr::NotShadowCaster, prelude::*};
use bevy_egui::EguiContextSettings;
use pool::spawn_pool;
use std::f32::consts::PI;
use sub::{spawn_sub, spawn_zed};

use crate::{control::PrimaryCamera, hal::ImageExportSource};

use super::{ViewCamera, physics::WaterCollider};

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
        PrimaryCamera,
        EguiContextSettings::default(),
        Name::new("Main cam"),
    ));

    // Pool
    let outer_half_size = Vec3::new(20., 6., 20.);
    let inner_half_size = Vec3::new(18., 5., 18.);
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
        // Cameras being inside meshes breaks subviews
        // Mesh3d(meshes.add(water_cuboid)),
        // MeshMaterial3d(water_material.clone()),
        // NotShadowCaster,
        Transform::default(),
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
