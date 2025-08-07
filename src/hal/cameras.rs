use async_io::Timer as AsyncTimer;
use bevy::render::camera::ExtractedCamera;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::{Render, RenderApp, RenderSet};
use bevy::time::Timer;
use std::sync::mpsc::channel;
use std::time::{Duration, SystemTime};

use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{Maintain, MapMode};
use bevy::tasks::IoTaskPool;
use bevy::{prelude::*, render::renderer::RenderDevice};
use futures_lite::FutureExt as _;

use super::BotCamImage;
use super::net::{OutgoingMessage, send};
use super::{ImageExportSource, ZedImage, image_export::GpuImageExportSource};

#[derive(Debug, Default, Clone)]
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<CameraTimer>::default(),
            ExtractComponentPlugin::<ZedCamera>::default(),
            ExtractComponentPlugin::<BottomCamera>::default(),
        ))
        .add_systems(PreUpdate, update_cam_timers)
        .add_systems(PostUpdate, update_cam_enabled)
        .register_type::<(CameraTimer, CameraEnabled, ZedCamera, BottomCamera)>();

        let render_app = app.sub_app_mut(RenderApp);

        render_app.add_systems(
            Render,
            (send_zed_image, send_botcam_image)
                .after(RenderSet::Render)
                .before(RenderSet::Cleanup),
        );
    }
}

#[derive(Debug, Clone, Component, Reflect, ExtractComponent)]
#[reflect(Debug, Clone, Component)]
pub struct CameraTimer {
    timer: Timer,
}

impl CameraTimer {
    pub fn from_rate(hz: f32) -> Self {
        Self {
            timer: Timer::new(Duration::from_secs_f32(1.0 / hz), TimerMode::Repeating),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Component, Reflect, ExtractComponent)]
#[reflect(Debug, Clone, Component)]
pub struct CameraEnabled(pub bool);

#[derive(Debug, Default, Clone, Copy, Component, Reflect, ExtractComponent)]
#[reflect(Component, Debug)]
#[require(CameraTimer::from_rate(ZED_FRAME_RATE), CameraEnabled)]
pub struct ZedCamera;

pub const ZED_FRAME_RATE: f32 = 20.0;

#[derive(Debug, Default, Clone, Component, Reflect, ExtractComponent)]
#[reflect(Component, Debug)]
#[require(CameraTimer::from_rate(BOT_CAM_FRAME_RATE), CameraEnabled)]
pub struct BottomCamera;

pub const BOT_CAM_FRAME_RATE: f32 = 20.0;

fn update_cam_timers(cameras: Query<&mut CameraTimer>, time: Res<Time<Fixed>>) {
    let dt = time.delta();
    for mut camera in cameras {
        camera.timer.tick(dt);
    }
}

pub fn update_cam_enabled(cameras: Query<(&CameraEnabled, &CameraTimer, &mut Camera)>) {
    for (enabled, timer, mut cam) in cameras {
        cam.is_active = enabled.0 && timer.timer.just_finished();
    }
}

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

// TODO: better rate limiting
pub fn send_zed_image(
    zed_cam: Query<(), (With<ExtractedCamera>, With<ZedCamera>)>,
    zed_image: Option<Res<ZedImage>>,
    sources: Res<RenderAssets<GpuImageExportSource>>,
    render_device: Res<RenderDevice>,
) -> Result {
    if zed_cam.is_empty() {
        return Ok(());
    }
    let Some(zed_image) = zed_image else {
        return Ok(());
    };
    let image = get_image(&zed_image.0, &*sources, &*render_device)?;
    IoTaskPool::get()
        .spawn(
            async move { send(OutgoingMessage::ZedImage(SystemTime::now(), image)).await }.or(
                async {
                    AsyncTimer::after(Duration::from_secs_f32(1.0)).await;
                    Err("Cancelled".into())
                },
            ),
        )
        .detach();

    Ok(())
}

pub fn send_botcam_image(
    bot_cam: Query<(), (With<ExtractedCamera>, With<BottomCamera>)>,
    botcam_image: Option<Res<BotCamImage>>,
    sources: Res<RenderAssets<GpuImageExportSource>>,
    render_device: Res<RenderDevice>,
) -> Result {
    if bot_cam.is_empty() {
        return Ok(());
    }
    let Some(botcam_image) = botcam_image else {
        return Ok(());
    };
    let image = get_image(&botcam_image.0, &*sources, &*render_device)?;
    IoTaskPool::get()
        .spawn(
            async move { send(OutgoingMessage::BotcamImage(SystemTime::now(), image)).await }.or(
                async {
                    AsyncTimer::after(Duration::from_secs_f32(1.0)).await;
                    Err("Cancelled".into())
                },
            ),
        )
        .detach();

    Ok(())
}
