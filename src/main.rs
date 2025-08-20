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
use control::{ControlState, ControllerPlugin};
use frustum_gizmo::FrustumGizmoPlugin;
use hal::HalPlugin;
use sim::{SimPlugin, sub::TeleopState};
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
        .add_systems(Startup, ui)
        .add_systems(
            OnEnter(ControlState::Focused),
            |q: Query<&mut Text, With<ModeText>>| enter(q, "Freecam"),
        )
        .add_systems(
            OnEnter(TeleopState::Teleop),
            |q: Query<&mut Text, With<ModeText>>| enter(q, "Teleop"),
        )
        .add_systems(
            OnEnter(TeleopState::NoTeleop),
            |q: Query<&mut Text, With<ModeText>>| enter(q, "Observer"),
        )
        .run();
}

#[derive(Debug, Component)]
struct ModeText;

fn ui(mut commands: Commands) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::FlexEnd,
            justify_content: JustifyContent::FlexEnd,
            ..default()
        },
        children![(
            Node {
                width: Val::Px(180.0),
                height: Val::Px(40.0),
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor(Color::WHITE),
            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
            children![(
                Text::new("Button"),
                TextFont {
                    font_size: 33.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextShadow::default(),
                ModeText
            )]
        )],
    ));
}

fn enter(nodes: Query<&mut Text, With<ModeText>>, new_text: &'static str) {
    for mut text in nodes {
        *text = new_text.into();
    }
}
