use async_io::Timer;
use std::sync::mpsc::channel;
use std::time::Duration;

use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{Maintain, MapMode};
use bevy::tasks::IoTaskPool;
use bevy::{prelude::*, render::renderer::RenderDevice};
use futures_lite::FutureExt as _;
use std::ops::Deref;

use crate::{BottomCamera, ZedCamera};

use super::BotCamImage;
use super::net::{OutgoingMessage, send};
use super::{ImageExportSource, ZedImage, image_export::GpuImageExportSource};

#[derive(Debug, Default, Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<u8>,
}

fn get_image(
    image: &Handle<ImageExportSource>,
    sources: &RenderAssets<GpuImageExportSource>,
    render_device: &RenderDevice,
) -> Result<Image> {
    let gpu_source = sources.get(image).ok_or("Image does not exist")?;
    let width = gpu_source.source_size.width;
    let height = gpu_source.source_size.height;
    let image_len = (width * height * 4) as usize;
    let image_bytes = {
        let slice = gpu_source.buffer.slice(..);

        {
            let (tx, rx) = channel();
            render_device.map_buffer(&slice, MapMode::Read, move |res| {
                tx.send(res).unwrap();
            });

            render_device.poll(Maintain::Wait);
            rx.recv()??;
        }

        assert_eq!(image_len, slice.get_mapped_range().len());
        slice.get_mapped_range().to_vec()
    };

    gpu_source.buffer.unmap();
    Ok(Image {
        width,
        height,
        buffer: image_bytes,
    })
}

pub fn save_zed_image(
    zed_cam: Query<&Camera, With<ZedCamera>>,
    zed_image: Option<Res<ZedImage>>,
    sources: Res<RenderAssets<GpuImageExportSource>>,
    render_device: Res<RenderDevice>,
) -> Result {
    if !zed_cam.iter().any(|cam| cam.is_active) {
        return Ok(());
    }
    let Some(zed_image) = zed_image else {
        return Ok(());
    };
    let image = get_image(&zed_image.0, &*sources, &*render_device)?;
    IoTaskPool::get()
        .spawn(
            async move { send(OutgoingMessage::ZedImage(image)).await }.or(async {
                Timer::after(Duration::from_secs_f32(1.0)).await;
                Err("Cancelled".into())
            }),
        )
        .detach();

    Ok(())
}

pub fn save_botcam_image(
    bot_cam: Query<&Camera, With<BottomCamera>>,
    botcam_image: Option<Res<BotCamImage>>,
    sources: Res<RenderAssets<GpuImageExportSource>>,
    render_device: Res<RenderDevice>,
) -> Result {
    if !bot_cam.iter().any(|cam| cam.is_active) {
        return Ok(());
    }
    let Some(botcam_image) = botcam_image else {
        return Ok(());
    };
    let image = get_image(&botcam_image.0, &*sources, &*render_device)?;
    IoTaskPool::get()
        .spawn(
            async move { send(OutgoingMessage::BotcamImage(image)).await }.or(async {
                Timer::after(Duration::from_secs_f32(1.0)).await;
                Err("Cancelled".into())
            }),
        )
        .detach();

    Ok(())
}
