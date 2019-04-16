use crate::{mutaction::MutationBuilder, DatabaseRead, DatabaseUpdate, DatabaseWrite, Sqlite};
use connector::{
    filter::{Filter, NodeSelector},
    ConnectorResult,
};
use prisma_models::{GraphqlId, ModelRef, PrismaArgs, PrismaListValue, RelationFieldRef};
use rusqlite::Transaction;
use std::sync::Arc;

impl DatabaseUpdate for Sqlite {
    fn execute_update(
        conn: &Transaction,
        node_selector: &NodeSelector,
        non_list_args: &PrismaArgs,
        list_args: &[(String, PrismaListValue)],
    ) -> ConnectorResult<GraphqlId> {
        let model = node_selector.field.model();
        let id = Self::id_for(conn, node_selector)?;
        let updating = MutationBuilder::update_one(Arc::clone(&model), &id, non_list_args)?;

        if let Some(update) = updating {
            Self::execute_one(conn, update)?;
        }

        Self::update_list_args(conn, vec![id.clone()], Arc::clone(&model), list_args.to_vec())?;

        Ok(id)
    }

    fn execute_update_many(
        conn: &Transaction,
        model: ModelRef,
        filter: &Filter,
        non_list_args: &PrismaArgs,
        list_args: &[(String, PrismaListValue)],
    ) -> ConnectorResult<usize> {
        let ids = Self::ids_for(conn, Arc::clone(&model), filter.clone())?;
        let count = ids.len();

        let updates = {
            let ids: Vec<&GraphqlId> = ids.iter().map(|id| &*id).collect();
            MutationBuilder::update_many(Arc::clone(&model), ids.as_slice(), non_list_args)?
        };

        Self::execute_many(conn, updates)?;
        Self::update_list_args(conn, ids, Arc::clone(&model), list_args.to_vec())?;

        Ok(count)
    }

    fn execute_nested_update(
        conn: &Transaction,
        parent_id: &GraphqlId,
        node_selector: &Option<NodeSelector>,
        relation_field: RelationFieldRef,
        non_list_args: &PrismaArgs,
        list_args: &[(String, PrismaListValue)],
    ) -> ConnectorResult<GraphqlId> {
        if let Some(ref node_selector) = node_selector {
            Self::id_for(conn, node_selector)?;
        };

        let id = Self::get_id_by_parent(conn, Arc::clone(&relation_field), parent_id, node_selector)?;

        let node_selector = NodeSelector::from((relation_field.related_model().fields().id(), id));
        Self::execute_update(conn, &node_selector, non_list_args, list_args)
    }

    fn execute_nested_update_many(
        conn: &Transaction,
        parent_id: &GraphqlId,
        filter: &Option<Filter>,
        relation_field: RelationFieldRef,
        non_list_args: &PrismaArgs,
        list_args: &[(String, PrismaListValue)],
    ) -> ConnectorResult<usize> {
        let ids = Self::get_ids_by_parents(conn, Arc::clone(&relation_field), vec![parent_id], filter.clone())?;
        let count = ids.len();

        let updates = {
            let ids: Vec<&GraphqlId> = ids.iter().map(|id| &*id).collect();
            MutationBuilder::update_many(relation_field.related_model(), ids.as_slice(), non_list_args)?
        };

        Self::execute_many(conn, updates)?;
        Self::update_list_args(conn, ids, relation_field.model(), list_args.to_vec())?;

        Ok(count)
    }

    fn update_list_args(
        conn: &Transaction,
        ids: Vec<GraphqlId>,
        model: ModelRef,
        list_args: Vec<(String, PrismaListValue)>,
    ) -> ConnectorResult<()> {
        for (field_name, list_value) in list_args {
            let field = model.fields().find_from_scalar(&field_name).unwrap();
            let table = field.scalar_list_table();
            let (deletes, inserts) = MutationBuilder::update_scalar_list_values(&table, &list_value, ids.to_vec());

            Self::execute_many(conn, deletes)?;
            Self::execute_many(conn, inserts)?;
        }

        Ok(())
    }
}
