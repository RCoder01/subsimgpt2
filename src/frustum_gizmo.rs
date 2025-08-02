use bevy::app::{App, Plugin, PostUpdate};
use bevy::color::{Color, Oklcha};
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::{
    component::Component,
    entity::Entity,
    query::Without,
    reflect::ReflectComponent,
    system::{Query, Res},
};
use bevy::gizmos::AppGizmoBuilder as _;
use bevy::gizmos::{
    config::{GizmoConfigGroup, GizmoConfigStore},
    gizmos::Gizmos,
};
use bevy::math::Vec3;
use bevy::reflect::{Reflect, ReflectFromReflect, std_traits::ReflectDefault};
use bevy::render::primitives::HalfSpace;
use bevy::render::{primitives::Frustum, view::VisibilitySystems};

/// Plugin for the drawing of [`Frustum`]s.
#[derive(Default, Copy, Clone)]
pub struct FrustumGizmoPlugin;

impl Plugin for FrustumGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<FrustumGizmoConfigGroup>()
            .init_gizmo_group::<FrustumGizmoConfigGroup>()
            .add_systems(
                PostUpdate,
                (
                    draw_frustums,
                    draw_all_frustums.run_if(|config: Res<GizmoConfigStore>| {
                        config.config::<FrustumGizmoConfigGroup>().1.draw_all
                    }),
                )
                    .after(VisibilitySystems::UpdateFrusta),
            );
    }
}

/// Configuration for drawing the [`Frustum`] component on entities.
#[derive(Clone, Default, Reflect, GizmoConfigGroup)]
#[reflect(Clone, Default)]
pub struct FrustumGizmoConfigGroup {
    /// Draws all frusta in the scene when set to `true`.
    ///
    /// To draw a specific entity's frustum, you can add the [`FrustumGizmo`] component.
    ///
    /// Defaults to `false`.
    pub draw_all: bool,
    /// The default color for frustum gizmos.
    ///
    /// A random color is chosen per frustum if `None`.
    ///
    /// Defaults to `None`.
    pub default_color: Option<Color>,
}

/// Add this [`Component`] to an entity to draw its [`Frustum`] component.
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component, FromReflect, Default)]
pub struct ShowFrustumGizmo {
    /// The color of the frustum.
    ///
    /// The default color from the [`GizmoConfig`] resource is used if `None`,
    pub color: Option<Color>,
}

fn draw_frustums(
    query: Query<(Entity, &Frustum, &ShowFrustumGizmo)>,
    mut gizmos: Gizmos<FrustumGizmoConfigGroup>,
) {
    for (entity, frustum, gizmo) in query {
        let color = gizmo
            .color
            .or(gizmos.config_ext.default_color)
            .unwrap_or_else(|| color_from_entity(entity));
        draw_frustum(frustum, color, &mut gizmos);
    }
}

fn draw_all_frustums(
    query: Query<(Entity, &Frustum), Without<ShowFrustumGizmo>>,
    mut gizmos: Gizmos<FrustumGizmoConfigGroup>,
) {
    for (entity, frustum) in &query {
        let color = gizmos
            .config_ext
            .default_color
            .unwrap_or_else(|| color_from_entity(entity));
        draw_frustum(frustum, color, &mut gizmos);
    }
}

fn draw_frustum(frustum: &Frustum, color: Color, gizmos: &mut Gizmos<FrustumGizmoConfigGroup>) {
    let Some([tln, trn, brn, bln, tlf, trf, brf, blf]) = frustum_corners(frustum) else {
        return;
    };

    let strip_positions = [
        tln, trn, brn, bln, tln, // Near
        tlf, trf, brf, blf, tlf, // Far
    ];
    gizmos.linestrip(strip_positions, color);

    gizmos.line(trn, trf, color);
    gizmos.line(brn, brf, color);
    gizmos.line(bln, blf, color);
}

fn frustum_corners(frustum: &Frustum) -> Option<[Vec3; 8]> {
    let [left, right, top, bottom, near, far] = frustum.half_spaces;
    Some([
        halfspace_intersect(top, left, near)?,
        halfspace_intersect(top, right, near)?,
        halfspace_intersect(bottom, right, near)?,
        halfspace_intersect(bottom, left, near)?,
        halfspace_intersect(top, left, far)?,
        halfspace_intersect(top, right, far)?,
        halfspace_intersect(bottom, right, far)?,
        halfspace_intersect(bottom, left, far)?,
    ])
}

fn halfspace_intersect(a: HalfSpace, b: HalfSpace, c: HalfSpace) -> Option<Vec3> {
    let an = a.normal();
    let bn = b.normal();
    let cn = c.normal();

    let x = Vec3::new(an.x, bn.x, cn.x);
    let y = Vec3::new(an.y, bn.y, cn.y);
    let z = Vec3::new(an.z, bn.z, cn.z);

    let d = -Vec3::new(a.d(), b.d(), c.d());

    let u = y.cross(z);
    let v = x.cross(d);

    let denom = x.dot(u);

    if denom.abs() < f32::EPSILON {
        return None;
    }

    Some(Vec3::new(d.dot(u), z.dot(v), -y.dot(v)) / denom)
}

fn color_from_entity(entity: Entity) -> Color {
    Oklcha::sequential_dispersed(entity.index()).into()
}
