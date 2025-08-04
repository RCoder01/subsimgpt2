use bevy::prelude::*;

use crate::sim::{
    BottomCamera, ZedCamera,
    sub::thruster::{ThrusterOf, ThrusterTarget},
};

use super::net::IncomingMessage;

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
                target.target_speed = powers[info.id as usize];
            }
        }
    }
}

pub fn handle_cameras(
    mut incoming: EventReader<IncomingMessage>,
    bottom_cameras: Query<&mut Camera, (With<BottomCamera>, Without<ZedCamera>)>,
    zed_cameras: Query<&mut Camera, (With<ZedCamera>, Without<BottomCamera>)>,
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
            cam.is_active = *new_active;
        }
    }
    if let Some(new_active) = zed_cam_on {
        for mut cam in zed_cameras {
            info!("Setting zed to {new_active}");
            cam.is_active = *new_active;
        }
    }
    Ok(())
}
