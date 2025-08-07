use bevy::{
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseMotion, MouseWheel},
    },
    math::DVec2,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};

#[derive(Debug, Default, Clone, Copy)]
pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MovementScale(0.2))
            .init_state::<ControlState>()
            .add_systems(
                Update,
                (
                    keyboard_input,
                    camera_look,
                    check_unconfine_system,
                    recenter_mouse,
                    camera_speed,
                )
                    .run_if(in_state(ControlState::Focused)),
            )
            .add_systems(
                Update,
                check_confine_system.run_if(in_state(ControlState::Unfocused)),
            )
            .register_type::<PrimaryCamera>()
            .register_type::<MovementScale>();
    }
}

#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct PrimaryCamera;

#[derive(Resource, Reflect)]
#[reflect(Resource)]
struct MovementScale(f32);

#[derive(Debug, States, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ControlState {
    #[default]
    Unfocused,
    Focused,
}

fn recenter_mouse(
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
    mut last_mouse_pos: Local<Option<Vec2>>,
) {
    let mut window = primary_window.single_mut().unwrap();
    if window.cursor_options.grab_mode == CursorGrabMode::Confined && last_mouse_pos.is_none() {
        *last_mouse_pos = window.cursor_position();
    }
    if window.cursor_options.grab_mode != CursorGrabMode::Confined {
        *last_mouse_pos = None;
        return;
    }
    if let Some(last_pos) = *last_mouse_pos {
        window.set_physical_cursor_position(Some(DVec2::new(last_pos.x as f64, last_pos.y as f64)));
    }
}

fn check_confine_system(
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
) {
    if mouse_button.just_pressed(MouseButton::Middle) || keyboard_input.just_pressed(KeyCode::Tab) {
        let mut window = primary_window.single_mut().unwrap();
        window.cursor_options.grab_mode = CursorGrabMode::Confined;
        window.cursor_options.visible = false;
        commands.insert_resource(NextState::Pending(ControlState::Focused));
    }
}

fn check_unconfine_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    if keyboard_input.pressed(KeyCode::Escape) || keyboard_input.just_pressed(KeyCode::Tab) {
        let mut window = primary_window.single_mut().unwrap();
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
        commands.insert_resource(NextState::Pending(ControlState::Unfocused));
    }
}

fn camera_look(
    mut camera: Query<&mut Transform, With<PrimaryCamera>>,
    mut mouse_movement: EventReader<MouseMotion>,
) {
    if let Ok(mut camera_transform) = camera.single_mut() {
        for event in mouse_movement.read() {
            camera_transform.rotation *= Quat::from_rotation_x(-0.002 * event.delta.y)
                * Quat::from_rotation_y(-0.002 * event.delta.x);
        }

        // Ensure roll is 0
        let euler = camera_transform.rotation.to_euler(EulerRot::YXZ);
        let new_quat = Quat::from_euler(EulerRot::YXZ, euler.0, euler.1, 0.);
        camera_transform.rotation = new_quat;
    }
}

fn keyboard_input(
    speed: Res<MovementScale>,
    mut camera: Query<&mut Transform, With<PrimaryCamera>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if let Ok(mut camera_transform) = camera.single_mut() {
        let scale = speed.0 * (keyboard_input.pressed(KeyCode::End) as i32 as f32 + 1.);
        let transform = *camera_transform;
        if keyboard_input.pressed(KeyCode::KeyW) {
            camera_transform.translation += transform.forward() * scale;
        };
        if keyboard_input.pressed(KeyCode::KeyS) {
            camera_transform.translation += transform.back() * scale;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            camera_transform.translation += transform.left() * scale;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            camera_transform.translation += transform.right() * scale;
        }
        if keyboard_input.pressed(KeyCode::KeyE) {
            camera_transform.translation += transform.up() * scale;
        }
        if keyboard_input.pressed(KeyCode::KeyQ) {
            camera_transform.translation += transform.down() * scale;
        }
        if keyboard_input.pressed(KeyCode::Space) {
            camera_transform.translation += Vec3::Y * scale;
        }
        if keyboard_input.pressed(KeyCode::ShiftLeft) {
            camera_transform.translation += -Vec3::Y * scale;
        }
    }
}

fn camera_speed(
    mut speed: ResMut<MovementScale>,
    mut scroll_event: EventReader<MouseWheel>,
    mut key_event: EventReader<KeyboardInput>,
) {
    for event in scroll_event.read() {
        if event.y >= 1. {
            speed.0 *= 1.5;
        }
        if event.y <= -1. {
            speed.0 /= 1.5;
        }
    }
    for event in key_event.read() {
        match event.key_code {
            KeyCode::ArrowUp => speed.0 *= 1.5,
            KeyCode::ArrowDown => speed.0 /= 1.5,
            _ => {}
        }
    }
}
