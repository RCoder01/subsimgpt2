use avian3d::{
    parry::na::SVector,
    prelude::{ComputedCenterOfMass, ExternalForce},
};
use bevy::prelude::*;

use crate::utils::add_forces;

#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, PartialEq, Debug)]
#[relationship(relationship_target = Thrusters)]
pub struct ThrusterOf {
    #[relationship]
    pub sub: Entity,
    pub id: u8,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect, Deref)]
#[reflect(Component, PartialEq, Debug)]
#[relationship_target(relationship = ThrusterOf)]
pub struct Thrusters(Vec<Entity>);

#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, PartialEq, Debug)]
#[require(ThrusterState, ThrusterParams)]
pub struct ThrusterTarget {
    pub target_output: f32,
}

#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, PartialEq, Debug)]
#[require(ThrusterForce)]
pub struct ThrusterState {
    output: f32,
}

#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, PartialEq, Debug)]
pub struct ThrusterForce {
    force: f32,
}

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, PartialEq, Debug)]
pub struct ThrusterParams {
    ease_factor: f32,
}

impl Default for ThrusterParams {
    fn default() -> Self {
        Self {
            ease_factor: 0.2, // Selected by hand
        }
    }
}

impl ThrusterParams {
    const POS_OUTPUT_FIT_CONSTANTS: [f32; 10] = [
        9.53215518e-01,
        1.10333738e00,
        -3.30202018e-01,
        -1.99208517e00,
        1.03546119e-01,
        2.82231393e-02,
        -8.42297013e-01,
        4.47743134e-01,
        -6.28233791e-03,
        -7.06918724e-04,
    ];
    const NEG_OUTPUT_FIT_CONSTANTS: [f32; 10] = [
        -1.91033060e00,
        -1.05256570e00,
        4.56828578e-01,
        -1.58452791e-01,
        2.59661426e-01,
        -3.32207446e-02,
        -1.07869796e00,
        -2.58917607e-01,
        -8.14000406e-03,
        7.63760288e-04,
    ];

    fn parameter(&self, output: f32) -> [f32; 10] {
        let a = output;
        let b = 14.8; // Voltage
        [
            1.0,
            a,
            b,
            a.powi(2),
            a * b,
            b.powi(2),
            a.powi(3),
            a.powi(2) * b,
            a * b.powi(2),
            b.powi(3),
        ]
    }

    fn model(&self, state: &ThrusterState) -> f32 {
        if (-0.05..0.05).contains(&state.output) {
            return 0.0;
        }
        let param = SVector::from(self.parameter(state.output));
        let coefs = if state.output > 0.0 {
            &SVector::from(Self::POS_OUTPUT_FIT_CONSTANTS)
        } else {
            &SVector::from(Self::NEG_OUTPUT_FIT_CONSTANTS)
        };
        param.dot(coefs)
    }
}

pub fn update_thruster_states(
    thrusters: Query<(&ThrusterTarget, &mut ThrusterState, &ThrusterParams)>,
) {
    for (target, mut state, params) in thrusters {
        let diff = target.target_output.clamp(-1.0, 1.0) - state.output;
        // TODO: Make this time independent
        // Current solution: run this system in FixedUpdate
        state.output += diff * params.ease_factor;
    }
}

pub fn update_thruster_forces(
    thrusters: Query<(&ThrusterState, &mut ThrusterForce, &ThrusterParams)>,
) {
    for (state, mut force, params) in thrusters {
        force.force = params.model(state);
    }
}

pub fn debug_thruster_states(
    thrusters: Query<(
        &GlobalTransform,
        &ThrusterTarget,
        &ThrusterState,
        &ThrusterForce,
    )>,
    mut gizmos: Gizmos,
) {
    for (transform, target, state, force) in thrusters {
        let start = transform.translation();
        let forward = transform.up() * 0.2;
        let target_arrow = target.target_output * forward;
        gizmos.arrow(start, start + target_arrow, Srgba::RED);
        let state_arrow = state.output * forward;
        gizmos.arrow(start, start + state_arrow, Srgba::GREEN);
        let force_arrow = force.force * forward;
        gizmos.arrow(start, start + force_arrow, Srgba::BLUE);
    }
}

pub fn thruster_physics(
    thrusters: Query<(&GlobalTransform, &ThrusterForce, &ThrusterParams)>,
    subs: Query<(&GlobalTransform, &Thrusters, Entity, &ComputedCenterOfMass)>,
    mut commands: Commands,
) -> Result {
    for (sub_transform, sub_thrusters, sub_entity, com) in subs {
        let mut force = ExternalForce::default();
        let sub_com = sub_transform.transform_point(com.0);
        for &thruster in &**sub_thrusters {
            let (thruster_transform, state, params) = thrusters.get(thruster)?;
            let thruster_center = thruster_transform.translation();
            let thruster_force = thruster_transform.up() * state.force;
            force.apply_force_at_point(thruster_force, thruster_center, sub_com);
        }
        commands
            .entity(sub_entity)
            .entry::<ExternalForce>()
            .or_default()
            .and_modify(move |mut ef| {
                *ef = add_forces(&*ef, &force, false);
            });
    }
    Ok(())
}
