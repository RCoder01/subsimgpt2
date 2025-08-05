mod pool;
mod sub;

use avian3d::prelude::{Collider, RigidBody};
use bevy::{pbr::NotShadowCaster, prelude::*, render::view::RenderLayers};
use bevy_egui::EguiContextSettings;
use pool::spawn_pool;
use std::f32::consts::{FRAC_PI_2, PI};
use sub::{SubEntity, spawn_sub};

use crate::{
    control::PrimaryCamera,
    hal::{ImageExportSource, MLTargetKind, MLTargetOf},
};

use super::{GIZMO_RENDER_LAYER, ViewCamera, WATER_RENDER_LAYER, physics::WaterCollider};

pub fn startup_spawner(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut export_sources: ResMut<Assets<ImageExportSource>>,
    asset_server: Res<AssetServer>,
) {
    let water_material = materials.add(StandardMaterial {
        perceptual_roughness: 0.0,
        specular_transmission: 1.0,
        base_color: Color::srgb(0.2, 0.5, 0.7),
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: None,
        // thickness: 0.2,
        // ior: 1.33,
        // attenuation_distance: 1.,
        // attenuation_color: Color::srgb(1.0, 1.0, 1.0),
        // diffuse_transmission: 0.5,
        ..default()
    });

    // Camera
    commands.spawn((
        Camera3d::default(),
        RenderLayers::default() | GIZMO_RENDER_LAYER | WATER_RENDER_LAYER,
        ViewCamera,
        PrimaryCamera,
        EguiContextSettings::default(),
        Name::new("Main cam"),
    ));

    // Pool
    let outer_half_size = Vec3::new(6.5, 1.5, 6.5);
    let inner_half_size = Vec3::new(6., 1., 6.);
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
        // Translucent materials break subviews
        WATER_RENDER_LAYER,
        Transform::default(),
        WaterCollider(water_cuboid),
        Name::new("Water"),
    ));

    // Sub
    let SubEntity { sub, zed_left } = spawn_sub(
        &mut *meshes,
        &mut *materials,
        &mut commands,
        &mut *images,
        &mut *export_sources,
    );

    // Light
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 1.0, 1.0, -PI / 4.)),
        Name::new("Sun"),
    ));

    gate(
        &mut commands,
        &mut materials,
        &mut meshes,
        &asset_server,
        zed_left,
    );
}

fn inches(e: f32) -> f32 {
    e * 0.0254
}

fn inches3(v: Vec3) -> Vec3 {
    v * 0.0254
}

fn gate(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    asset_server: &AssetServer,
    zed_cam: Entity,
) {
    let pvc_white = materials.add(Color::from(Srgba::WHITE));
    let foam_red = materials.add(Color::from(Srgba::new(0.7, 0.1, 0.1, 1.0)));
    let foam_black = materials.add(Color::from(Srgba::new(0.1, 0.1, 0.1, 1.0)));

    let top_bar_shape = Cylinder::new(inches(1.05) / 2.0, inches(120.0));
    let top_bar = meshes.add(top_bar_shape);

    let side_half_bar_shape = Cuboid::new(inches(3.), inches(24.), inches(3.));
    let side_half_bar = meshes.add(side_half_bar_shape);

    let divider_shape = Cuboid::new(inches(0.5), inches(24.), inches(2.));
    let divider = meshes.add(divider_shape);

    let image_shape = Rectangle::new(inches(12.), inches(12.));
    let image = meshes.add(image_shape);
    let image_cube = Cuboid::from_size(image_shape.size().extend(inches(0.3)));
    let left_image_material = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/gate_image_left.png")),
        cull_mode: None,
        ..default()
    });
    let right_image_material = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/gate_image_right.png")),
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Transform::from_translation(Vec3::new(5.0, 0.0, 0.0)),
        Visibility::default(),
        RigidBody::Static,
        Name::new("Gate"),
        children![
            (
                Transform::from_rotation(Quat::from_axis_angle(Vec3::X, FRAC_PI_2)),
                Mesh3d(top_bar),
                MeshMaterial3d(pvc_white.clone()),
                Collider::from(top_bar_shape),
                Name::new("Gate top bar"),
            ),
            (
                Transform::from_translation(inches3(Vec3::new(0., -13., -60.))),
                Mesh3d(side_half_bar.clone()),
                MeshMaterial3d(foam_black.clone()),
                Collider::from(side_half_bar_shape),
                Name::new("Gate left bar top")
            ),
            (
                Transform::from_translation(inches3(Vec3::new(0.0, -37., -60.))),
                Mesh3d(side_half_bar.clone()),
                MeshMaterial3d(foam_red.clone()),
                Collider::from(side_half_bar_shape),
                Name::new("Gate left bar bottom")
            ),
            (
                Transform::from_translation(inches3(Vec3::new(0.0, -12., 0.0))),
                Mesh3d(divider),
                MeshMaterial3d(foam_red.clone()),
                Collider::from(divider_shape),
                Name::new("Gate divider")
            ),
            (
                Transform::from_translation(inches3(Vec3::new(0., -13., 60.))),
                Mesh3d(side_half_bar.clone()),
                MeshMaterial3d(foam_red.clone()),
                Collider::from(side_half_bar_shape),
                Name::new("Gate right bar top")
            ),
            (
                Transform::from_translation(inches3(Vec3::new(0.0, -37., 60.))),
                Mesh3d(side_half_bar.clone()),
                MeshMaterial3d(foam_black.clone()),
                Collider::from(side_half_bar_shape),
                Name::new("Gate right bar bottom")
            ),
            (
                Transform::from_translation(inches3(Vec3::new(0.0, -6., -30.)))
                    .with_rotation(Quat::from_axis_angle(Vec3::Y, FRAC_PI_2)),
                Mesh3d(image.clone()),
                MeshMaterial3d(left_image_material),
                Collider::from(image_cube),
                MLTargetOf {
                    target_camera: zed_cam,
                    shape: image_cube,
                    kind: MLTargetKind::GateRed,
                },
                Name::new("Left image")
            ),
            (
                Transform::from_translation(inches3(Vec3::new(0.0, -6., 30.)))
                    .with_rotation(Quat::from_axis_angle(Vec3::Y, FRAC_PI_2)),
                Mesh3d(image.clone()),
                MeshMaterial3d(right_image_material),
                Collider::from(image_cube),
                MLTargetOf {
                    target_camera: zed_cam,
                    shape: image_cube,
                    kind: MLTargetKind::GateBlue,
                },
                Name::new("Left image")
            ),
        ],
    ));
}
