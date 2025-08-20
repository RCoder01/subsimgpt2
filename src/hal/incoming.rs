use avian3d::prelude::{
    Collider, ColliderOf, ColliderTransform, LinearVelocity, PhysicsGizmoExt, PhysicsGizmos,
    RigidBodyColliders,
};
use bevy::prelude::*;

use crate::sim::sub::{
    SubControls,
    thruster::{ThrusterOf, ThrusterTarget},
};

use super::{BottomCamera, CameraEnabled, ZedCamera, net::IncomingMessage};

pub fn handle_thrusters(
    mut incoming: EventReader<IncomingMessage>,
    thrusters: Query<(&mut ThrusterTarget, &ThrusterOf)>,
) {
    let mut powers = None;
    for message in incoming.read() {
        let IncomingMessage::Motors(new_speeds) = message else {
            continue;
        };
        powers = Some(new_speeds);
    }
    if let Some(powers) = powers {
        for (mut target, info) in thrusters {
            if (info.id as usize) < powers.len() {
                target.target_output = powers[info.id as usize];
            }
        }
    }
}

pub fn handle_cameras(
    mut incoming: EventReader<IncomingMessage>,
    bottom_cameras: Query<&mut CameraEnabled, (With<BottomCamera>, Without<ZedCamera>)>,
    zed_cameras: Query<&mut CameraEnabled, (With<ZedCamera>, Without<BottomCamera>)>,
) -> Result {
    let mut bot_cam_on = None;
    let mut zed_cam_on = None;
    for message in incoming.read() {
        match message {
            IncomingMessage::BotcamOn(new_active) => {
                bot_cam_on = Some(new_active);
            }
            IncomingMessage::ZedOn(new_active) => {
                zed_cam_on = Some(new_active);
            }
            _ => {}
        }
    }
    if let Some(new_active) = bot_cam_on {
        for mut cam in bottom_cameras {
            info!("Setting botcam to {new_active}");
            cam.0 = *new_active;
        }
    }
    if let Some(new_active) = zed_cam_on {
        for mut cam in zed_cameras {
            info!("Setting zed to {new_active}");
            cam.0 = *new_active;
        }
    }
    Ok(())
}

#[derive(Debug, Component)]
pub struct LocalizationEstimate;

pub fn update_localization_estimate(
    mut incoming: EventReader<IncomingMessage>,
    mut estimate: Query<(&mut Transform, &mut LinearVelocity), With<LocalizationEstimate>>,
    mut commands: Commands,
) {
    let transforms = incoming.read().filter_map(|m| {
        let IncomingMessage::LocalizationEstimate {
            rotation,
            position,
            velocity,
        } = m
        else {
            return None;
        };
        let change_of_coordinates = Mat3::from_cols_array(&[1., 0., 0., 0., 0., 1., 0., -1., 0.]);
        let translation = change_of_coordinates * *position;
        let rotation = Quat::from_mat3(rotation);
        Some((
            Transform {
                translation,
                rotation,
                scale: Vec3::ONE,
            },
            change_of_coordinates * *velocity,
        ))
    });
    let Some((new_transform, new_vel)) = transforms.last() else {
        return;
    };
    if let Ok((mut transform, mut velocity)) = estimate.single_mut() {
        *transform = new_transform;
        velocity.0 = new_vel;
        return;
    }
    commands.spawn((
        LocalizationEstimate,
        new_transform,
        LinearVelocity(new_vel),
        Name::new("Localization estimate"),
    ));
}

pub fn debug_localization(
    estimate: Query<(&Transform, &LinearVelocity), With<LocalizationEstimate>>,
    mut gizmos: Gizmos<PhysicsGizmos>,
    sub: Query<&RigidBodyColliders, With<SubControls>>,
    colliders: Query<(&Collider, &ColliderTransform), With<ColliderOf>>,
) -> Result<()> {
    let Ok((estimate_transform, estimate_vel)) = estimate.single() else {
        return Ok(());
    };
    let Ok(sub_colliders) = sub.single() else {
        return Ok(());
    };

    const COLOR: Color = Color::srgba(1.0, 0.5, 0.0, 1.0);
    gizmos.arrow(
        estimate_transform.translation,
        estimate_transform.translation + estimate_vel.0,
        COLOR,
    );

    for entity in sub_colliders.iter() {
        let (collider, relative) = colliders.get(entity)?;
        let relative_transform = Transform {
            translation: relative.translation,
            rotation: relative.rotation.0,
            scale: Vec3::ONE,
        };
        let transform = *estimate_transform * relative_transform;
        gizmos.draw_collider(collider, transform.translation, transform.rotation, COLOR);
    }

    Ok(())
}
