mod cameras;
mod image_export;
mod incoming;
mod net;

use bevy::{
    prelude::*,
    render::{Render, RenderApp, RenderSet},
};
use cameras::{save_botcam_image, save_zed_image};
pub use image_export::{BotCamImage, ImageExportSource, ZedImage};
use incoming::{handle_cameras, handle_thrusters};

#[derive(Debug, Default, Clone)]
pub struct HalPlugin;

impl Plugin for HalPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins((image_export::ImageExportPlugin, net::NetPlugin))
            .add_systems(Update, (handle_thrusters, handle_cameras));

        let render_app = app.sub_app_mut(RenderApp);

        render_app.add_systems(
            Render,
            (save_zed_image, save_botcam_image)
                .after(RenderSet::Render)
                .before(RenderSet::Cleanup),
        );
    }
}
