use core::f32::consts::FRAC_PI_2;

use avian3d::prelude::{
    AngularDamping, AngularInertia, AngularInertiaTensor, CenterOfMass, Collider, ExternalForce,
    LinearDamping, Mass, RigidBody,
};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        camera::{RenderTarget, Viewport},
        render_resource::{self, TextureDimension, TextureFormat, TextureUsages},
    },
};

use crate::{
    frustum_gizmo::ShowFrustumGizmo,
    hal::{
        BotCamImage, BottomCamera, CameraEnabled, CameraTimer, ImageExportSource, MLTargets,
        Sensors, ZedCamera, ZedImage,
    },
    sim::{
        physics::{BuoyancySamples, SubBuoyancy, WaterResistance},
        sub::{
            SubControls,
            thruster::{ThrusterOf, ThrusterTarget, Thrusters},
        },
    },
};

const SUB_SIZE: Vec3 = Vec3::new(0.35, 0.15, 0.35);

pub struct SubEntity {
    pub sub: Entity,
    pub zed_left: Entity,
    // pub zed_right: Entity,
    // pub bot_cam: Entity,
}

pub fn spawn_sub(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    export_sources: &mut Assets<ImageExportSource>,
) -> SubEntity {
    commands.insert_resource(BuoyancySamples::new(1000));
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
            Transform::from_translation(Vec3::new(1., -1., 0.)),
            Name::new("Sub"),
            SubControls::new(0.4),
            Collider::from(sub_cuboid),
            RigidBody::Dynamic,
            SubBuoyancy::new(sub_cuboid, 1.01),
            // TODO: Make this vary based on % underwater
            (
                WaterResistance {
                    factor: 5.0,
                    cuboid: sub_cuboid,
                },
                AngularDamping(0.6),
            ),
            //
            ExternalForce::ZERO.with_persistence(false),
            CenterOfMass(com),
            Mass(25.),
            // TODO: What might the tensor's value be?
            AngularInertia::from(AngularInertiaTensor::default() / 0.2),
            Thrusters::default(),
            Sensors::default(),
        ))
        .id();
    let ZedCamEntity { left } = spawn_zed(commands, sub_entity, images, export_sources);
    spawn_botcam(commands, sub_entity, images, export_sources);

    let thruster_material = materials.add(StandardMaterial {
        base_color: Srgba::BLACK.into(),
        ..default()
    });
    let thruster_mesh = Extrusion::new(Annulus::new(0.05, 0.06), 0.11)
        .mesh()
        .build()
        .rotated_by(Quat::from_axis_angle(Vec3::X, FRAC_PI_2));
    let thruster_mesh = meshes.add(thruster_mesh);
    let base_translation = com * 0.4;
    let base_rotation = Quat::from_axis_angle(Vec3::Z, -FRAC_PI_2);
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
            Name::new("Thruster"),
        ));
    }

    SubEntity {
        sub: sub_entity,
        zed_left: left,
    }
}

struct ZedCamEntity {
    left: Entity,
    // right: Entity,
}

fn spawn_zed(
    commands: &mut Commands,
    sub: Entity,
    images: &mut Assets<Image>,
    export_sources: &mut Assets<ImageExportSource>,
) -> ZedCamEntity {
    let mut image = Image::new_fill(
        render_resource::Extent3d {
            width: 1280,
            height: 480,
            ..default()
        },
        TextureDimension::D2,
        &[0, 255, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.label = Some("Zed cam target");
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC;
    let target = images.add(image);
    commands.insert_resource(ZedImage(export_sources.add(target.clone())));
    // left camera
    let left = commands
        .spawn((
            Camera3d::default(),
            Camera {
                viewport: Some(Viewport {
                    physical_position: UVec2::new(0, 0),
                    physical_size: UVec2::new(640, 480),
                    ..default()
                }),
                target: RenderTarget::Image(target.into()),
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: 70.0,
                ..default()
            }),
            Transform::from_translation(Vec3::new(0.08, 0.0, -0.03))
                .with_rotation(Quat::from_axis_angle(Vec3::Y, -FRAC_PI_2)),
            ShowFrustumGizmo::default(),
            ZedCamera::default(),
            CameraEnabled(false),
            ChildOf(sub),
            MLTargets::default(),
            Name::new("Left Zed cam"),
        ))
        .id();
    ZedCamEntity { left }
}

fn spawn_botcam(
    commands: &mut Commands,
    sub: Entity,
    images: &mut Assets<Image>,
    export_sources: &mut Assets<ImageExportSource>,
) -> Entity {
    let mut image = Image::new_fill(
        render_resource::Extent3d {
            width: 960,
            height: 540,
            ..default()
        },
        TextureDimension::D2,
        &[0, 255, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.label = Some("Bot cam image target");
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC;
    let target = images.add(image);
    commands.insert_resource(BotCamImage(export_sources.add(target.clone())));
    commands
        .spawn((
            Camera3d::default(),
            Camera {
                target: RenderTarget::Image(target.into()),
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: 127.0,
                ..default()
            }),
            Transform::from_translation(Vec3::new(0., -0.01, 0.)).with_rotation(
                Quat::from_rotation_y(-FRAC_PI_2) * Quat::from_rotation_x(-FRAC_PI_2),
            ),
            ShowFrustumGizmo::default(),
            BottomCamera::default(),
            CameraEnabled(false),
            ChildOf(sub),
            Name::new("Bottom camera"),
        ))
        .id()
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
            self.pitch.to_radians(),
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
