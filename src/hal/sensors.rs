use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, RigidBody, Rotation};
use bevy::prelude::*;

#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
pub struct PreviousVelocity(pub Vec3);

#[derive(Debug, Default, Clone, Bundle)]
pub struct Sensors {
    dvl: Dvl,
    imu: Imu,
    depth_sensor: DepthSensor,
}

#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
#[require(RigidBody)]
pub struct Dvl {
    pub velocity: Vec3,
}

#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
#[require(RigidBody, PreviousVelocity)]
pub struct Imu {
    pub angle: Quat,
    pub angular_velocity: Vec3,
    pub dvel: Vec3,
    pub dt: f32,
}

#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
#[require(RigidBody)]
pub struct DepthSensor {
    pub depth: f32,
}

pub fn update_previous_velocities(
    mut imus: Query<(&LinearVelocity, &mut PreviousVelocity), With<RigidBody>>,
) {
    for (vel, mut prev_vel) in imus.iter_mut() {
        prev_vel.0 = vel.0;
    }
}

// TODO: Noise models
// TODO: Relative offsets
pub fn postupdate_sensors(
    mut dvls: Query<(&LinearVelocity, &mut Dvl), With<RigidBody>>,
    mut imus: Query<
        (
            &LinearVelocity,
            &PreviousVelocity,
            &AngularVelocity,
            &Rotation,
            &mut Imu,
        ),
        With<RigidBody>,
    >,
    mut depths: Query<(&Position, &mut DepthSensor), With<RigidBody>>,
    time: Res<Time<Fixed>>,
) {
    for (vel, mut dvl) in dvls.iter_mut() {
        dvl.velocity = vel.0;
    }
    for (vel, prev_vel, ang_vel, rot, mut imu) in imus.iter_mut() {
        info!("{:.10}", vel.0);
        info!("{:.10}", prev_vel.0);
        info!("{:.10}", vel.0 - prev_vel.0);
        let dt = time.delta_secs();
        *imu = Imu {
            angle: rot.0,
            angular_velocity: ang_vel.0,
            dvel: vel.0 - prev_vel.0,
            dt,
        }
    }
    for (pos, mut depth) in depths.iter_mut() {
        depth.depth = -pos.y;
    }
}
