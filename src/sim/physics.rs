use avian3d::prelude::{ComputedCenterOfMass, ComputedMass, ExternalForce, Gravity, PhysicsSet};
use bevy::prelude::*;
use rand::{SeedableRng as _, rngs::StdRng};

use crate::utils::add_forces;

use super::sub::thruster::ThrusterOf;

#[derive(Debug, Default, Clone, Copy)]
pub struct SubPhysicsPlugin;

impl Plugin for SubPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            |thrusters: Query<&GlobalTransform, With<ThrusterOf>>, mut gizmos: Gizmos| {
                for transform in thrusters {
                    gizmos.axes(transform.compute_transform(), 0.05)
                }
            },
        )
        .add_systems(FixedUpdate, buoyancy.in_set(SubPhysicsSet))
        .configure_sets(FixedUpdate, SubPhysicsSet.before(PhysicsSet::Prepare))
        .init_resource::<BuoyancySamples>()
        .register_type::<SubPhysicsSet>()
        .register_type::<WaterCollider>()
        .register_type::<SubBuoyancy>()
        .register_type::<BuoyancySamples>();
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect, Hash, SystemSet)]
pub struct SubPhysicsSet;

#[derive(Debug, Component, Reflect, Deref, Clone, Copy)]
#[reflect(Component, Debug)]
pub struct WaterCollider(pub Cuboid);

#[derive(Debug, Component, Reflect, Deref, Clone, Copy)]
#[reflect(Component)]
pub struct Buoyant<S: ShapeSample> {
    #[deref]
    shape: S,
    buoyancy_factor: f32,
}

impl<S: ShapeSample> Buoyant<S> {
    pub fn new(shape: S, buoyancy_factor: f32) -> Self {
        Self {
            shape,
            buoyancy_factor,
        }
    }
}

pub type SubBuoyancy = Buoyant<Cuboid>;

#[derive(Debug, Clone, Copy, Resource, Reflect, PartialEq, Eq)]
#[reflect(Resource, PartialEq, Debug)]
pub struct BuoyancySamples {
    pub count: u32,
}

impl BuoyancySamples {
    pub fn new(count: u32) -> Self {
        Self { count }
    }
}

impl Default for BuoyancySamples {
    fn default() -> Self {
        Self::new(100)
    }
}

fn buoyancy(
    gravity: Res<Gravity>,
    samples: Res<BuoyancySamples>,
    subs: Query<
        (
            &GlobalTransform,
            &SubBuoyancy,
            &ComputedMass,
            &ComputedCenterOfMass,
            Entity,
        ),
        With<ExternalForce>,
    >,
    water: Query<(&GlobalTransform, &WaterCollider)>,
    mut gizmos: Gizmos,
    mut commands: Commands,
    mut rng: Local<Option<StdRng>>,
) -> Result {
    // TODO: Is this a sufficiently deterministic rng?
    let mut rng = rng.get_or_insert_with(|| StdRng::from_seed([0; 32]));
    let (water_transform, water_cuboid) = water.single()?;
    let water_inverse = water_transform.affine().inverse();
    for (transform, buoyancy, mass, com, entity) in subs {
        let mut force = ExternalForce::default();
        let gravity_force = gravity.0 * mass.value();
        let global_com = transform.transform_point(com.0);

        let force_per_sample = -gravity_force / (samples.count as f32) * buoyancy.buoyancy_factor;
        for _ in 0..samples.count {
            let local_sample = buoyancy.sample_interior(&mut rng);
            let global_sample = transform.transform_point(local_sample);
            let water_local = water_inverse.transform_point(global_sample);
            let underwater = water_cuboid.closest_point(water_local) == water_local;
            if underwater {
                force.apply_force_at_point(force_per_sample, global_sample, global_com);
                // gizmos.arrow(
                //     global_sample,
                //     global_sample + force_per_sample * 0.1,
                //     Srgba::GREEN,
                // );
            } else {
                // gizmos.arrow(
                //     global_sample,
                //     global_sample + force_per_sample * 0.1,
                //     Srgba::RED,
                // );
            }
        }
        commands
            .entity(entity)
            .entry::<ExternalForce>()
            .or_default()
            .and_modify(move |mut ef| {
                *ef = add_forces(&*ef, &force, false);
            });

        gizmos.arrow(global_com, transform.translation(), Srgba::BLUE);
    }
    Ok(())
}
