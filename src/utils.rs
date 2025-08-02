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
