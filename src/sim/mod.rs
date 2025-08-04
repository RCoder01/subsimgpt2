pub mod physics;
pub mod scene;
pub mod sub;

use bevy::{
    prelude::*,
    render::extract_component::{ExtractComponent, ExtractComponentPlugin},
    window::{PresentMode, PrimaryWindow},
};
use physics::SubPhysicsPlugin;
use scene::startup_spawner;
use sub::SubPlugin;

#[derive(Debug, Default, Clone, Copy)]
pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SubPhysicsPlugin::default(),
            SubPlugin::default(),
            ExtractComponentPlugin::<ZedCamera>::default(),
            ExtractComponentPlugin::<BottomCamera>::default(),
        ))
        .add_systems(Startup, (startup_spawner, disable_vsync))
        .add_systems(Update, |mut gizmos: Gizmos| {
            gizmos.axes(Transform::default(), 1.)
        })
        .register_type::<ViewCamera>()
        .register_type::<ZedCamera>();
    }
}
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Debug)]
pub struct ViewCamera;

#[derive(Debug, Clone, Copy, Component, Reflect, ExtractComponent)]
#[reflect(Component, Debug)]
pub struct ZedCamera;

#[derive(Debug, Clone, Copy, Component, Reflect, ExtractComponent)]
#[reflect(Component, Debug)]
pub struct BottomCamera;

fn disable_vsync(mut window: Query<&mut Window, With<PrimaryWindow>>) -> Result {
    let mut window = window.single_mut()?;
    window.present_mode = PresentMode::AutoNoVsync;
    Ok(())
}
