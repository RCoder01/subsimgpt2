mod control;
mod frustum_gizmo;
pub mod hal;
pub mod sim;
mod skybox;
mod utils;

use avian3d::{PhysicsPlugins, prelude::PhysicsDebugPlugin};
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use control::ControllerPlugin;
use frustum_gizmo::FrustumGizmoPlugin;
use hal::HalPlugin;
use sim::SimPlugin;
use skybox::SkyboxPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            bevy_framepace::FramepacePlugin,
            EguiPlugin::default(),
            WorldInspectorPlugin::new(),
            LogDiagnosticsPlugin::default(),
            // FrameTimeDiagnosticsPlugin::default(),
            SkyboxPlugin,
            PhysicsPlugins::default(),
            FrustumGizmoPlugin::default(),
            PhysicsDebugPlugin::default(),
            HalPlugin::default(),
            ControllerPlugin::default(),
            SimPlugin::default(),
        ))
        .run();
}
