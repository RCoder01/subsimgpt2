pub mod physics;
pub mod scene;
pub mod sub;

use avian3d::prelude::PhysicsGizmos;
use bevy::{
    prelude::*,
    render::view::RenderLayers,
    window::{PresentMode, PrimaryWindow},
};
use physics::SubPhysicsPlugin;
use scene::startup_spawner;
use sub::SubPlugin;

use crate::frustum_gizmo::FrustumGizmoConfigGroup;

#[derive(Debug, Default, Clone, Copy)]
pub struct SimPlugin;

pub const GIZMO_RENDER_LAYER: RenderLayers = RenderLayers::layer(1);
pub const WATER_RENDER_LAYER: RenderLayers = RenderLayers::layer(2);

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        let config = GizmoConfig {
            render_layers: GIZMO_RENDER_LAYER,
            ..default()
        };
        app.add_plugins((SubPhysicsPlugin::default(), SubPlugin::default()))
            .add_systems(Startup, (startup_spawner, disable_vsync))
            .add_systems(Update, |mut gizmos: Gizmos| {
                gizmos.axes(Transform::default(), 0.2)
            })
            .insert_gizmo_config(
                PhysicsGizmos {
                    axis_lengths: Some(Vec3::splat(0.2)),
                    ..default()
                },
                config.clone(),
            )
            .insert_gizmo_config(DefaultGizmoConfigGroup::default(), config.clone())
            .insert_gizmo_config(LightGizmoConfigGroup::default(), config.clone())
            .insert_gizmo_config(FrustumGizmoConfigGroup::default(), config.clone());
    }
}
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Debug)]
pub struct ViewCamera;

fn disable_vsync(mut window: Query<&mut Window, With<PrimaryWindow>>) -> Result {
    let mut window = window.single_mut()?;
    window.present_mode = PresentMode::AutoNoVsync;
    Ok(())
}
