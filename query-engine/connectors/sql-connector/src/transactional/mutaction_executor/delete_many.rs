use crate::{
    mutaction::{DeleteActions, MutationBuilder},
    Transaction,
};
use connector::{filter::Filter, ConnectorResult};
use prisma_models::{GraphqlId, ModelRef, RelationFieldRef};
use std::sync::Arc;

/// A top level delete that removes records matching the `Filter`. Violating
/// any relations will cause an error.
///
/// Will return the number records deleted.
pub fn execute(conn: &mut Transaction, model: ModelRef, filter: &Filter) -> ConnectorResult<usize> {
    let ids = conn.filter_ids(Arc::clone(&model), filter.clone())?;
    let ids: Vec<&GraphqlId> = ids.iter().map(|id| &*id).collect();
    let count = ids.len();

    DeleteActions::check_relation_violations(Arc::clone(&model), ids.as_slice(), |select| {
        let ids = conn.select_ids(select)?;
        Ok(ids.into_iter().next())
    })?;

    for delete in MutationBuilder::delete_many(model, ids.as_slice()) {
        conn.delete(delete)?;
    }

    Ok(count)
}

/// Removes nested items matching to filter, or if no filter is given, all
/// nested items related to the given `parent_id`. An error will be thrown
/// if any deleted record is required in a model.
pub fn execute_nested(
    conn: &mut Transaction,
    parent_id: &GraphqlId,
    filter: &Option<Filter>,
    relation_field: RelationFieldRef,
) -> ConnectorResult<usize> {
    let ids = conn.filter_ids_by_parents(Arc::clone(&relation_field), vec![parent_id], filter.clone())?;
    let count = ids.len();

    let ids: Vec<&GraphqlId> = ids.iter().map(|id| &*id).collect();
    let model = relation_field.model();

    DeleteActions::check_relation_violations(model, ids.as_slice(), |select| {
        let ids = conn.select_ids(select)?;
        Ok(ids.into_iter().next())
    })?;

    for delete in MutationBuilder::delete_many(relation_field.related_model(), ids.as_slice()) {
        conn.delete(delete)?;
    }

    Ok(count)
}
