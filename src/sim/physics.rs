use arrayvec::ArrayVec;
use avian3d::prelude::{
    ComputedCenterOfMass, ComputedMass, ExternalForce, Gravity, LinearDamping, LinearVelocity,
    PhysicsSet,
};
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
        .add_systems(
            FixedUpdate,
            (buoyancy, linear_damping).in_set(SubPhysicsSet),
        )
        .configure_sets(FixedUpdate, SubPhysicsSet.before(PhysicsSet::Prepare))
        .init_resource::<BuoyancySamples>()
        .register_type::<(
            SubPhysicsSet,
            WaterCollider,
            SubBuoyancy,
            BuoyancySamples,
            WaterResistance,
        )>();
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
            }
            // gizmos.arrow(
            //     global_sample,
            //     global_sample + force_per_sample * 0.1,
            //     if underwater { Srgba::GREEN } else { Srgba::RED },
            // );
        }
        commands
            .entity(entity)
            .entry::<ExternalForce>()
            .or_default()
            .and_modify(move |mut ef| {
                *ef = add_forces(&ef, &force, false);
            });

        gizmos.arrow(global_com, transform.translation(), Srgba::BLUE);
    }
    Ok(())
}

#[derive(Debug, Component, Reflect, Clone)]
#[reflect(Component, Debug)]
#[require(LinearDamping)]
pub struct WaterResistance {
    pub factor: f32,
    pub cuboid: Cuboid,
}

fn projected_area(triangle: Triangle3d, target_normal: Vec3) -> f32 {
    Triangle3d {
        vertices: triangle
            .vertices
            .map(|p| p.reject_from_normalized(target_normal)),
    }
    .area()
}

// Find the point along the ray from start to end that intersects the y=0 plane
fn find_intersect(start: Vec3, end: Vec3) -> Result<Vec3> {
    let ray = Ray3d::new(start, Dir3::new(end - start)?);
    let intersect = ray
        .intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
        .ok_or("Ray doesn't intersect y=0")?;
    Ok(ray.get_point(intersect))
}

/// Trims/splits the triangle so that all vertices are at or below y=0
/// Produces 0, 1, or 2 subtriangles
fn trim_triangle(triangle: Triangle3d) -> Result<ArrayVec<Triangle3d, 2>> {
    let mut trims = ArrayVec::<Triangle3d, 2>::new_const();
    let pred = |p: &Vec3| p.y <= 0.0;
    let below_zero_count = triangle.vertices.iter().copied().filter(pred).count();
    match below_zero_count {
        0 => {}
        1 => {
            let below_index = triangle
                .vertices
                .iter()
                .position(pred)
                .ok_or("below_zero_count > 0")?;
            let prev_index = (below_index + 3 - 1) % 3;
            let next_index = (below_index + 1) % 3;
            let mut trimmed = triangle;
            trimmed.vertices[prev_index] = find_intersect(
                triangle.vertices[below_index],
                triangle.vertices[prev_index],
            )?;
            trimmed.vertices[next_index] = find_intersect(
                triangle.vertices[below_index],
                triangle.vertices[next_index],
            )?;
            trims.push(trimmed);
        }
        2 => {
            let above_index = triangle
                .vertices
                .iter()
                .position(|p| !pred(p))
                .expect("3 - below_zero_count > 0");
            let prev_index = (above_index + 3 - 1) % 3;
            let next_index = (above_index + 1) % 3;
            let prev_corner = triangle.vertices[prev_index];
            let new_prev_corner = find_intersect(triangle.vertices[above_index], prev_corner)?;
            let next_corner = triangle.vertices[next_index];
            let new_next_corner = find_intersect(triangle.vertices[above_index], next_corner)?;
            trims.push(Triangle3d {
                vertices: [prev_corner, new_prev_corner, new_next_corner],
            });
            trims.push(Triangle3d {
                vertices: [prev_corner, new_next_corner, next_corner],
            });
        }
        3 => {
            trims.push(triangle);
        }
        _ => unreachable!(),
    }
    Ok(trims)
}

/// Assumes water level is y = 0
// projects the cuboid mesh into the velocity direction and calculates the area
fn linear_damping(
    subs: Query<(
        &GlobalTransform,
        &WaterResistance,
        &LinearVelocity,
        &mut LinearDamping,
    )>,
) -> Result {
    for (transform, resistance, velocity, mut damping) in subs {
        let Some(vel_dir) = velocity.try_normalize() else {
            continue;
        };
        let mut total_area = 0.0;
        // resistance could be any other convex mesh, not just cuboids
        for triangle in resistance
            .cuboid
            .mesh()
            .build()
            .triangles()
            .expect("Cuboid-built mesh should be ok")
        {
            // So that we don't double count the surface area
            if !triangle.normal().is_ok_and(|dir| dir.dot(vel_dir) > 0.0) {
                continue;
            }
            let transformed_triangle = Triangle3d {
                vertices: triangle.vertices.map(|p| transform.transform_point(p)),
            };
            let trimmed = match trim_triangle(transformed_triangle) {
                Ok(trimmed) => trimmed,
                Err(e) => {
                    warn!("{e}");
                    continue;
                }
            };
            for sub_tri in trimmed {
                total_area += projected_area(sub_tri, vel_dir);
            }
        }
        damping.0 = resistance.factor * total_area;
    }
    Ok(())
}
