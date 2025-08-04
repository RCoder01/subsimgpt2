mod cameras;
mod image_export;
mod incoming;
mod net;
mod sensors;

use avian3d::{
    prelude::{PhysicsSet, RigidBody},
    prepare::PrepareSet,
};
use bevy::{
    prelude::*,
    render::{Render, RenderApp, RenderSet},
    tasks::IoTaskPool,
};
use cameras::{send_botcam_image, send_zed_image};
pub use image_export::{BotCamImage, ImageExportSource, ZedImage};
use incoming::{handle_cameras, handle_thrusters};
use net::{Dvl as DvlMessage, ImuINS, ImuPIMU, OutgoingMessage, SensorMessage, send};
use sensors::{postupdate_sensors, update_previous_velocities};

pub use sensors::{DepthSensor, Dvl, Imu, Sensors};

#[derive(Debug, Default, Clone)]
pub struct HalPlugin;

impl Plugin for HalPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins((image_export::ImageExportPlugin, net::NetPlugin))
            .add_systems(Update, (handle_thrusters, handle_cameras))
            .add_systems(
                FixedUpdate,
                (
                    update_previous_velocities.before(PrepareSet::InitTransforms),
                    postupdate_sensors.after(PhysicsSet::Sync),
                    send_sensors,
                )
                    .chain(),
            );

        let render_app = app.sub_app_mut(RenderApp);

        render_app.add_systems(
            Render,
            (send_zed_image, send_botcam_image)
                .after(RenderSet::Render)
                .before(RenderSet::Cleanup),
        );
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
            theta: [roll, pitch, -yaw], // TODO: Is this the correct order (and EulerRot)? (yes?)
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
