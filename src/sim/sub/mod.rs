pub mod thruster;

use std::f32::consts::{FRAC_PI_2, PI};

use avian3d::prelude::{AngularVelocity, LinearVelocity};
use bevy::prelude::*;
use rand::{Rng as _, thread_rng};
use thruster::{
    ThrusterForce, ThrusterOf, ThrusterParams, ThrusterState, ThrusterTarget, Thrusters,
    debug_thruster_states, thruster_physics, update_thruster_forces, update_thruster_states,
};

use crate::control::ControlState;

use super::physics::SubPhysicsSet;

#[derive(Debug, Default, Clone, Copy)]
pub struct SubPlugin;

impl Plugin for SubPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_thruster_states,
                update_thruster_forces.after(update_thruster_states),
                debug_thruster_states.after(update_thruster_forces),
                thruster_physics.after(update_thruster_forces),
            )
                .in_set(SubPhysicsSet),
        )
        .add_systems(
            Update,
            |thrusters: Query<&GlobalTransform, With<ThrusterOf>>, mut gizmos: Gizmos| {
                for transform in thrusters {
                    gizmos.axes(transform.compute_transform(), 0.05)
                }
            },
        )
        .add_systems(
            Update,
            (set_teleop_state, reset_sub, coin_flip_sub).run_if(in_state(ControlState::Unfocused)),
        )
        .add_systems(Update, sub_controls.run_if(in_state(TeleopState::Teleop)))
        .add_sub_state::<TeleopState>()
        .register_type::<(
            SubControls,
            ThrusterOf,
            Thrusters,
            ThrusterTarget,
            ThrusterState,
            ThrusterForce,
            ThrusterParams,
        )>();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(ControlState = ControlState::Unfocused)]
pub enum TeleopState {
    #[default]
    NoTeleop,
    Teleop,
}

fn set_teleop_state(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    state: Res<State<TeleopState>>,
    mut commands: Commands,
) {
    if keyboard_input.just_pressed(KeyCode::KeyT) {
        commands.insert_resource(NextState::Pending(match **state {
            TeleopState::NoTeleop => TeleopState::Teleop,
            TeleopState::Teleop => TeleopState::NoTeleop,
        }));
    }
}

#[derive(Debug, Component, Reflect)]
#[reflect(Component, Debug)]
pub struct SubControls {
    scale: f32,
}

impl SubControls {
    pub fn new(scale: f32) -> Self {
        Self { scale }
    }
}

fn sub_controls(
    mut thrusters: Query<(&ThrusterOf, &mut ThrusterTarget)>,
    subs: Query<(Entity, &SubControls)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) -> Result {
    let (sub, controls) = subs.single()?;
    let mut linear = Vec3::ZERO;
    let mut yaw = 0.0;
    if keyboard_input.pressed(KeyCode::KeyW) {
        linear += Vec3::X;
    };
    if keyboard_input.pressed(KeyCode::KeyS) {
        linear += Vec3::NEG_X;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        linear += Vec3::NEG_Z;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        linear += Vec3::Z;
    }
    if keyboard_input.pressed(KeyCode::Space) {
        linear += Vec3::NEG_Y;
    }
    if keyboard_input.pressed(KeyCode::ShiftLeft) {
        linear += Vec3::Y;
    }
    if keyboard_input.pressed(KeyCode::KeyQ) {
        yaw -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyE) {
        yaw += 1.0;
    }
    let mut max = 0.0_f32;
    for (info, mut target) in &mut thrusters {
        if info.sub != sub {
            continue;
        }
        target.target_output = match info.id {
            0 => linear.x - linear.z - yaw,
            1 => linear.x + linear.z + yaw,
            2 => linear.x + linear.z - yaw,
            3 => linear.x - linear.z + yaw,
            4..8 => -linear.y,
            _ => {
                panic!("Unexpected thruster id {} for sub {sub}", info.id)
            }
        } * controls.scale;
        max = max.max(target.target_output.abs());
    }
    if max < 1e-5 {
        return Ok(());
    }
    // let scale = controls.scale / max;
    // for (info, mut target) in thrusters {
    //     if info.sub != sub {
    //         continue;
    //     }
    //     target.target_speed *= scale;
    // }
    Ok(())
}

pub fn reset_sub(
    mut sub: Query<(&mut Transform, &mut LinearVelocity, &mut AngularVelocity), With<SubControls>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) -> Result<()> {
    if keyboard_input.just_pressed(KeyCode::KeyR) {
        let (mut transform, mut vel, mut ang_vel) = sub.single_mut()?;
        *transform = Transform::from_xyz(1., -0.2, 0.);
        *vel = default();
        *ang_vel = default();
    }
    Ok(())
}

pub fn coin_flip_sub(
    mut sub: Query<(&mut Transform, &mut LinearVelocity, &mut AngularVelocity), With<SubControls>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) -> Result<()> {
    if keyboard_input.just_pressed(KeyCode::KeyC) {
        let (mut transform, mut vel, mut ang_vel) = sub.single_mut()?;
        let mut rand = thread_rng();
        let coin_flip: bool = rand.r#gen();
        let angle = if coin_flip { -FRAC_PI_2 } else { -PI };
        *transform = Transform::from_xyz(1., -0.2, 0.).with_rotation(Quat::from_rotation_y(angle));
        *vel = default();
        *ang_vel = default();
    }
    Ok(())
}
