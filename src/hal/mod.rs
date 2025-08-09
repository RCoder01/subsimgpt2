mod cameras;
mod image_export;
mod incoming;
mod net;
mod sensors;
mod target;

use avian3d::{
    prelude::{PhysicsSet, RigidBody},
    prepare::PrepareSet,
};
use bevy::{prelude::*, tasks::IoTaskPool};
use cameras::update_cam_enabled;
pub use image_export::{BotCamImage, ImageExportSource, ZedImage};
use incoming::{handle_cameras, handle_thrusters};
use net::{Dvl as DvlMessage, ImuINS, ImuPIMU, OutgoingMessage, SensorMessage, send};
use sensors::{postupdate_sensors, update_previous_velocities};

pub use cameras::{BottomCamera, CameraEnabled, CameraTimer, ZedCamera};
pub use net::MLTargetKind;
pub use sensors::{DepthSensor, Dvl, Imu, Sensors};
pub use target::{MLTargetOf, MLTargets};
use target::{MLTargetSizeThreshold, send_ml_targets};

#[derive(Debug, Default, Clone)]
pub struct HalPlugin;

impl Plugin for HalPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins((
            image_export::ImageExportPlugin,
            net::NetPlugin,
            cameras::CameraPlugin,
        ))
        .add_systems(Update, (handle_thrusters, handle_cameras))
        .add_systems(
            FixedUpdate,
            (
                update_previous_velocities.before(PrepareSet::InitTransforms),
                postupdate_sensors.after(PhysicsSet::Sync),
                send_sensors,
            )
                .chain(),
        )
        .add_systems(PostUpdate, send_ml_targets.after(update_cam_enabled))
        .init_resource::<MLTargetSizeThreshold>()
        .register_type::<(MLTargets, MLTargetOf, MLTargetSizeThreshold)>();
    }
}

fn send_sensors(sub: Query<(&Dvl, &Imu, &DepthSensor), With<RigidBody>>) -> Result {
    let (dvl, imu, depth) = sub.single()?;
    let Imu {
        angle,
        angular_velocity,
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
            theta: [roll, pitch, yaw], // TODO: Is this the correct order (and EulerRot)? (yes?)
        },
        imu_pimu: ImuPIMU {
            dtheta: (angular_velocity / dt).to_array(),
            dvel: dvel.to_array(),
            dt: *dt,
        },
    };
    IoTaskPool::get()
        .spawn(async move { send(OutgoingMessage::Sensors(message)).await })
        .detach();
    Ok(())
}
