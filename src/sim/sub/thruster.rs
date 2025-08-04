use avian3d::prelude::{ComputedCenterOfMass, ExternalForce};
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
    pub target_speed: f32,
}

#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, PartialEq, Debug)]
pub struct ThrusterState {
    speed: f32,
}

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, PartialEq, Debug)]
pub struct ThrusterParams {
    max_force: f32,
    ease_factor: f32,
}

impl Default for ThrusterParams {
    fn default() -> Self {
        Self {
            max_force: 4.0,   // Approx max reverse force for a T200 thruster at 14.8 V
            ease_factor: 0.2, // Selected by hand
        }
    }
}

pub fn update_thruster_speeds(
    thrusters: Query<(&ThrusterTarget, &mut ThrusterState, &ThrusterParams)>,
) {
    for (target, mut state, params) in thrusters {
        let diff = target.target_speed - state.speed;
        // TODO: Make this time independent
        // Current solution: run this system in FixedUpdate
        state.speed += diff * params.ease_factor;
    }
}

pub fn debug_thruster_speeds(
    thrusters: Query<(&GlobalTransform, &ThrusterTarget, &ThrusterState)>,
    mut gizmos: Gizmos,
) {
    for (transform, target, state) in thrusters {
        let start = transform.translation();
        let target_arrow = target.target_speed * transform.up();
        gizmos.arrow(start, start + target_arrow, Srgba::RED);
        let state_arrow = state.speed * transform.up();
        gizmos.arrow(start, start + state_arrow, Srgba::GREEN);
    }
}

pub fn thruster_physics(
    thrusters: Query<(&GlobalTransform, &ThrusterState, &ThrusterParams)>,
    subs: Query<(&GlobalTransform, &Thrusters, Entity, &ComputedCenterOfMass)>,
    mut commands: Commands,
) -> Result {
    for (sub_transform, sub_thrusters, sub_entity, com) in subs {
        let mut force = ExternalForce::default();
        let sub_com = sub_transform.transform_point(com.0);
        for &thruster in &**sub_thrusters {
            let (thruster_transform, state, params) = thrusters.get(thruster)?;
            let thruster_center = thruster_transform.translation();
            let thruster_force = thruster_transform.up() * state.speed * params.max_force;
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
