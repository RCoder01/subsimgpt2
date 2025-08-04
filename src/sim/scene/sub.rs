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
    hal::{ImageExportSource, Sensors, ZedImage},
    sim::{
        ZedCamera,
        physics::SubBuoyancy,
        sub::{
            SubControls,
            thruster::{ThrusterOf, ThrusterTarget, Thrusters},
        },
    },
};

const SUB_SIZE: Vec3 = Vec3::new(0.35, 0.15, 0.35);

pub fn spawn_sub(
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
            Transform::from_translation(Vec3::new(0., -1., 0.)).with_scale(Vec3::splat(3.)),
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
            Sensors::default(),
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

pub fn spawn_zed(
    commands: &mut Commands,
    sub: Entity,
    images: &mut Assets<Image>,
    export_sources: &mut Assets<ImageExportSource>,
) {
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
    // right camera
    commands.spawn((
        Camera3d::default(),
        Camera {
            viewport: Some(Viewport {
                physical_position: UVec2::new(640, 0),
                physical_size: UVec2::new(640, 480),
                ..default()
            }),
            target: RenderTarget::Image(target.into()),
            is_active: true,
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
