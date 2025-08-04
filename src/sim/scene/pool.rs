use avian3d::prelude::{Collider, RigidBody};
use bevy::{pbr::NotShadowCaster, prelude::*};

pub fn spawn_pool(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    commands: &mut Commands,
    outer_half_size: Vec3,
    inner_half_size: Vec3,
) -> Entity {
    let o = outer_half_size;
    let i = inner_half_size;
    let a1 = Vec3::new(o[0], -o[1], -i[2]);
    let a2 = Vec3::new(-i[0], i[1], -o[2]);
    let b1 = Vec3::new(o[0], -o[1], -i[2]);
    let b2 = Vec3::new(i[0], i[1], o[2]);
    let c1 = Vec3::new(-o[0], -o[1], i[2]);
    let c2 = Vec3::new(i[0], i[1], o[2]);
    let d1 = Vec3::new(-o[0], -o[1], i[2]);
    let d2 = Vec3::new(-i[0], i[1], -o[2]);
    let f1 = -i;
    let f2 = Vec3::new(i[0], -o[1], i[2]);
    let material = MeshMaterial3d(materials.add(StandardMaterial { ..default() }));
    let mut wall = |x0, x1, name| {
        let cuboid = Cuboid::from_corners(x0, x1);
        let translation = Transform::from_translation(Vec3::midpoint(x0, x1));
        (
            Mesh3d(meshes.add(cuboid)),
            material.clone(),
            translation,
            Collider::from(cuboid),
            RigidBody::Static,
            NotShadowCaster,
            Name::new(name),
        )
    };
    commands
        .spawn((
            Name::new("Pool"),
            Transform::from_translation(Vec3::new(0., -i.y, 0.)),
            Visibility::default(),
            children![
                wall(a1, a2, "Wall0"),
                wall(b1, b2, "Wall1"),
                wall(c1, c2, "Wall2"),
                wall(d1, d2, "Wall3"),
                wall(f1, f2, "Floor"),
            ],
        ))
        .id()
}
