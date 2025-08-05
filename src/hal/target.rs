use bevy::math::primitives::Cuboid;
use bevy::prelude::*;
use bevy::render::primitives::Frustum;
use bevy::tasks::IoTaskPool;
use smallvec::SmallVec;

use super::net::{MLTargetData, MLTargetKind, OutgoingMessage, send};

#[derive(Debug, Clone, Copy, Component, Reflect)]
#[reflect(Debug, Clone, Component)]
#[relationship(relationship_target = MLTargets)]
pub struct MLTargetOf {
    #[relationship]
    pub target_camera: Entity,
    pub shape: Cuboid,
    pub kind: MLTargetKind,
}

#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Debug, Clone, Component)]
#[relationship_target(relationship = MLTargetOf)]
#[require(Frustum)]
pub struct MLTargets(Vec<Entity>);

fn cuboid_corners(cuboid: Cuboid) -> [Vec3; 8] {
    let p = cuboid.half_size;
    [
        p * Vec3::new(-1., -1., -1.),
        p * Vec3::new(-1., -1., 1.),
        p * Vec3::new(-1., 1., -1.),
        p * Vec3::new(-1., 1., 1.),
        p * Vec3::new(1., -1., -1.),
        p * Vec3::new(1., -1., 1.),
        p * Vec3::new(1., 1., -1.),
        p * Vec3::new(1., 1., 1.),
    ]
}

#[derive(Debug, Clone, Copy, Resource, Reflect)]
#[reflect(Debug, Clone, Resource)]
pub struct MLTargetSizeThreshold(Vec2);

impl Default for MLTargetSizeThreshold {
    fn default() -> Self {
        Self(Vec2::splat(50.))
    }
}

pub fn send_ml_targets(
    cameras: Query<(&Camera, &MLTargets, &GlobalTransform)>,
    targets: Query<(&MLTargetOf, &GlobalTransform)>,
    size_threshold: Res<MLTargetSizeThreshold>,
) -> Result<()> {
    let task_pool = IoTaskPool::get();
    for (cam, cam_targets, cam_transform) in cameras {
        let logical_rect = cam
            .logical_viewport_rect()
            .ok_or("MLTargets should have logical size")?;
        let mut detections: SmallVec<_> = default();
        if !cam.is_active {
            continue;
        }
        for target in &cam_targets.0 {
            let (target, transform) = targets.get(*target)?;
            let mut min = Vec2::MAX;
            let mut max = Vec2::MIN;
            for point in cuboid_corners(target.shape) {
                let world_pos = transform.transform_point(point);
                let Ok(logical) = cam.world_to_viewport(cam_transform, world_pos) else {
                    continue;
                };
                min = min.min(logical);
                max = max.max(logical);
            }
            min = min.clamp(logical_rect.min, logical_rect.max);
            max = max.clamp(logical_rect.min, logical_rect.max);
            let aabb = Rect::from_corners(min, max);
            if aabb.width() < size_threshold.0.x || aabb.height() < size_threshold.0.y {
                continue;
            }
            detections.push(MLTargetData {
                kind: target.kind,
                left: aabb.min.x,
                top: aabb.min.y,
                right: aabb.max.x,
                bottom: aabb.max.y,
            });
        }
        let message = OutgoingMessage::MlTarget(detections, logical_rect.size());
        task_pool
            .spawn(async move {
                let _ = send(message).await;
            })
            .detach();
    }
    Ok(())
}
