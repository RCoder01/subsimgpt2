#![allow(unused)]

use avian3d::prelude::ExternalForce;
use bevy::{
    ecs::query::{QueryData, QueryFilter, QueryItem, QuerySingleError, ROQueryItem},
    prelude::*,
};

pub fn zero_or_one<'a, D: QueryData, F: QueryFilter>(
    q: &'a Query<D, F>,
) -> Result<Option<ROQueryItem<'a, D>>> {
    match q.single() {
        Ok(single) => Ok(Some(single)),
        Err(QuerySingleError::NoEntities(_)) => Ok(None),
        Err(e @ QuerySingleError::MultipleEntities(_)) => Err(e.into()),
    }
}

pub fn zero_or_one_mut<'a, D: QueryData, F: QueryFilter>(
    q: &'a mut Query<D, F>,
) -> Result<Option<QueryItem<'a, D>>> {
    match q.single_mut() {
        Ok(single) => Ok(Some(single)),
        Err(QuerySingleError::NoEntities(_)) => Ok(None),
        Err(e @ QuerySingleError::MultipleEntities(_)) => Err(e.into()),
    }
}

pub fn add_forces(a: &ExternalForce, b: &ExternalForce, persistence: bool) -> ExternalForce {
    let force = a.force() + b.force();
    let torque = a.torque() + b.torque();
    (force, persistence, torque).into()
}

pub fn flatten_array<const M: usize, const N: usize, const O: usize>(
    input: [[u8; N]; M],
) -> [u8; O] {
    assert_eq!(M * N, O);
    let mut out = [0; O];
    for i in 0..M {
        out[(i * N)..((i + 1) * N)].copy_from_slice(&input[i]);
    }
    out
}
