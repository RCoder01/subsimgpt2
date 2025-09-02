mod cameras;
mod image_export;
mod incoming;
mod net;
mod sensors;
mod target;

use avian3d::prelude::PhysicsSet;
use bevy::{prelude::*, tasks::IoTaskPool};
use cameras::update_cam_enabled;
pub use image_export::{BotCamImage, ImageExportSource, ZedImage};
use incoming::{
    debug_localization, handle_cameras, handle_thrusters, update_localization_estimate,
};
use net::{Dvl as DvlMessage, ImuINS, ImuPIMU, OutgoingMessage, SensorMessage, send};
use sensors::{postupdate_sensors, send_sensors, update_previous_velocities};

pub use cameras::{BottomCamera, CameraEnabled, CameraTimer, ZedCamera};
pub use net::MLTargetKind;
pub use sensors::{DepthSensor, Dvl, Imu};
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
        .add_systems(
            Update,
            (
                handle_thrusters,
                handle_cameras,
                (update_localization_estimate, debug_localization).chain(),
            ),
        )
        .add_systems(
            FixedPostUpdate,
            (
                update_previous_velocities.before(PhysicsSet::Prepare),
                postupdate_sensors.after(PhysicsSet::Sync),
                send_sensors,
            )
                .chain(),
        )
        .add_systems(PostUpdate, send_ml_targets.after(update_cam_enabled))
        .init_resource::<MLTargetSizeThreshold>()
        .register_type::<(
            MLTargets,
            MLTargetOf,
            MLTargetSizeThreshold,
            Imu,
            Dvl,
            DepthSensor,
        )>();
    }
}
