use avian3d::prelude::{
    AngularVelocity, ComputedCenterOfMass, LinearVelocity, Position, RigidBody, Rotation,
};
use bevy::{prelude::*, tasks::IoTaskPool};

use crate::hal::net::{Dvl as DvlMessage, ImuINS, ImuPIMU, OutgoingMessage, SensorMessage, send};

#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
pub struct PreviousVelocity(pub Vec3);

#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
#[require(Transform)]
pub struct Dvl {
    pub velocity: Vec3,
}

#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
#[require(Transform, PreviousVelocity)]
pub struct Imu {
    pub angle: Quat,
    pub dtheta: [f32; 3],
    pub dvel: Vec3,
    pub dt: f32,
}

#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
#[require(Transform)]
pub struct DepthSensor {
    pub depth: f32,
}

pub fn update_previous_velocities(
    subs: Query<(
        &LinearVelocity,
        &AngularVelocity,
        &ComputedCenterOfMass,
        &GlobalTransform,
    )>,
    mut imus: Query<(&ChildOf, &GlobalTransform, &mut PreviousVelocity), With<RigidBody>>,
) -> Result {
    for (parent, transform, mut prev_vel) in imus.iter_mut() {
        let (lin_vel, ang_vel, com, parent_transform) = subs.get(parent.0)?;
        let offset = transform.translation() - *parent_transform * com.0;
        let vel = lin_vel.0 + ang_vel.cross(offset);
        prev_vel.0 = vel;
    }
    Ok(())
}

// TODO: Noise models
// TODO: Relative offsets
pub fn postupdate_sensors(
    mut dvls: Query<(&ChildOf, &GlobalTransform, &mut Dvl)>,
    mut imus: Query<(&ChildOf, &GlobalTransform, &PreviousVelocity, &mut Imu)>,
    mut depths: Query<(&GlobalTransform, &mut DepthSensor)>,
    subs: Query<
        (
            &LinearVelocity,
            &AngularVelocity,
            &Position,
            &Rotation,
            &ComputedCenterOfMass,
            &GlobalTransform,
        ),
        With<RigidBody>,
    >,
    time: Res<Time<Fixed>>,
) -> Result {
    for (parent, transform, mut dvl) in dvls.iter_mut() {
        let inverse_transform = transform.affine().inverse();
        let (lin_vel, _, ang_vel, _, com, parent_transform) = subs.get(parent.0)?;
        let offset = transform.translation() - *parent_transform * com.0;
        let vel = lin_vel.0 + ang_vel.cross(offset);
        dvl.velocity = inverse_transform.transform_vector3(vel);
    }
    for (parent, transform, prev_vel, mut imu) in imus.iter_mut() {
        let inverse_transform = transform.affine().inverse();
        let (lin_vel, ang_vel, _, rot, com, parent_transform) = subs.get(parent.0)?;
        let offset = transform.translation() - *parent_transform * com.0;
        let vel = lin_vel.0 + ang_vel.cross(offset);
        let dt = time.delta_secs();
        let mut rotation;
        if let Some(axis) = ang_vel.try_normalize() {
            let local_axis = inverse_transform.transform_vector3(axis);
            // has angle wrapping issues, but we shouldn't be exceeding a half rotation every tick
            let quat = Quat::from_axis_angle(local_axis, ang_vel.length() * dt);
            rotation = quat.to_euler(EulerRot::YZX).into();
        } else {
            rotation = [0.0; 3];
        }
        rotation[0] *= -1.;
        *imu = Imu {
            angle: rot.0,
            dtheta: rotation,
            dvel: inverse_transform.transform_vector3(vel - prev_vel.0),
            dt,
        }
    }
    for (pos, mut depth) in depths.iter_mut() {
        depth.depth = -pos.translation().y;
    }
    Ok(())
}

pub fn send_sensors(
    dvl: Query<(&ChildOf, &Dvl)>,
    imu: Query<(&ChildOf, &Imu)>,
    depth: Query<(&ChildOf, &DepthSensor)>,
) -> Result {
    let (e0, dvl) = dvl.single()?;
    let (e1, imu) = imu.single()?;
    let (e2, depth) = depth.single()?;
    assert_eq!(e0, e1);
    assert_eq!(e0, e2);
    let Imu {
        angle,
        dtheta,
        dvel,
        dt,
    } = imu;
    let (yaw, pitch, roll) = angle.to_euler(EulerRot::YZX);
    let message = SensorMessage {
        depth: depth.depth,
        dvl: DvlMessage {
            velocity_a: dvl.velocity.x,
            velocity_b: dvl.velocity.y,
            velocity_c: dvl.velocity.z,
        },
        imu_ins: ImuINS {
            theta: [-pitch, roll, yaw],
        },
        imu_pimu: ImuPIMU {
            dtheta: *dtheta,
            dvel: dvel.to_array(),
            dt: *dt,
        },
    };
    IoTaskPool::get()
        .spawn(async move { send(OutgoingMessage::Sensors(message)).await })
        .detach();
    Ok(())
}
