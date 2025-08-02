use std::{
    net::{TcpStream, ToSocketAddrs},
    sync::mpsc::channel,
    time::Duration,
};

use async_io::{Async, Timer};
use bevy::{
    asset::RenderAssetUsages,
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        graph::CameraDriverLabel,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel},
        render_resource::{
            Buffer, BufferDescriptor, BufferUsages, Extent3d, Maintain, MapMode,
            TexelCopyBufferInfo, TexelCopyBufferLayout,
        },
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
    },
    tasks::IoTaskPool,
};
use futures_lite::{AsyncWriteExt as _, FutureExt as _};

#[derive(Default, Debug)]
pub struct ImageExportPlugin;

impl Plugin for ImageExportPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(PostUpdate, ImageExportSystems::ImageExportSetup)
            .register_type::<ImageExportSource>()
            .init_asset::<ImageExportSource>()
            .register_asset_reflect::<ImageExportSource>()
            .add_plugins((
                RenderAssetPlugin::<GpuImageExportSource>::default(),
                ExtractResourcePlugin::<ZedImage>::default(),
                ExtractResourcePlugin::<BotCamImage>::default(),
            ));

        let render_app = app.sub_app_mut(RenderApp);

        let mut graph = render_app
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap();

        graph.add_node(ImageExportLabel, ImageExportNode);
        graph.add_node_edge(CameraDriverLabel, ImageExportLabel);
    }
}

#[derive(Debug, Clone, Resource, Reflect, ExtractResource, Deref)]
#[reflect(Debug, Clone, Resource)]
pub struct ZedImage(pub Handle<ImageExportSource>);

#[derive(Debug, Clone, Resource, Reflect, ExtractResource, Deref)]
#[reflect(Debug, Clone, Resource)]
pub struct BotCamImage(pub Handle<ImageExportSource>);

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct ImageExportLabel;

pub struct ImageExportNode;
impl Node for ImageExportNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        for (_, source) in world
            .resource::<RenderAssets<GpuImageExportSource>>()
            .iter()
        {
            if let Some(gpu_image) = world
                .resource::<RenderAssets<GpuImage>>()
                .get(&source.source_handle)
            {
                render_context.command_encoder().copy_texture_to_buffer(
                    gpu_image.texture.as_image_copy(),
                    TexelCopyBufferInfo {
                        buffer: &source.buffer,
                        layout: TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(source.padded_bytes_per_row),
                            rows_per_image: None,
                        },
                    },
                    source.source_size,
                );
            }
        }

        Ok(())
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum ImageExportSystems {
    ImageExportSetup,
}

#[derive(Asset, Reflect, Clone, Default)]
#[reflect(Clone, Default)]
pub struct ImageExportSource(pub Handle<Image>);

impl From<Handle<Image>> for ImageExportSource {
    fn from(value: Handle<Image>) -> Self {
        Self(value)
    }
}

pub struct GpuImageExportSource {
    pub buffer: Buffer,
    pub source_handle: Handle<Image>,
    pub source_size: Extent3d,
    pub bytes_per_row: u32,
    pub padded_bytes_per_row: u32,
}

impl RenderAsset for GpuImageExportSource {
    type SourceAsset = ImageExportSource;
    type Param = (SRes<RenderDevice>, SRes<RenderAssets<GpuImage>>);

    fn asset_usage(_: &Self::SourceAsset) -> RenderAssetUsages {
        RenderAssetUsages::RENDER_WORLD
    }

    fn prepare_asset(
        source_asset: Self::SourceAsset,
        _asset_id: AssetId<Self::SourceAsset>,
        (device, images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let gpu_image = images.get(&source_asset.0).unwrap();

        let size = gpu_image.texture.size();
        let format = &gpu_image.texture_format;
        let bytes_per_row =
            (size.width / format.block_dimensions().0) * format.block_copy_size(None).unwrap();
        let padded_bytes_per_row =
            RenderDevice::align_copy_bytes_per_row(bytes_per_row as usize) as u32;

        let source_size = gpu_image.texture.size();

        Ok(GpuImageExportSource {
            buffer: device.create_buffer(&BufferDescriptor {
                label: Some("Image Export Buffer"),
                size: (source_size.height * padded_bytes_per_row) as u64,
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            source_handle: source_asset.0,
            source_size,
            bytes_per_row,
            padded_bytes_per_row,
        })
    }

    fn byte_len(_: &Self::SourceAsset) -> Option<usize> {
        None
    }
}
