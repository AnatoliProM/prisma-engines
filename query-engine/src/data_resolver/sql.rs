use super::DataResolver;
use super::ScalarListValues;
use crate::protobuf::prelude::*;
use crate::{database_executor::DatabaseExecutor, node_selector::NodeSelector, query_builder::QueryBuilder};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use prisma_common::PrismaResult;
use prisma_models::prelude::*;
use rusqlite::Row;

pub struct SqlResolver<T>
where
    T: DatabaseExecutor,
{
    database_executor: T,
}

impl<T> DataResolver for SqlResolver<T>
where
    T: DatabaseExecutor,
{
    fn get_node_by_where(
        &self,
        node_selector: NodeSelector,
        selected_fields: SelectedFields,
    ) -> PrismaResult<Option<SingleNode>> {
        let (db_name, query) = QueryBuilder::get_node_by_where(node_selector, &selected_fields);
        let scalar_fields = selected_fields.scalar_non_list();
        let field_names = scalar_fields.iter().map(|f| f.name.clone()).collect();

        let nodes = self
            .database_executor
            .with_rows(query, db_name, |row| Self::read_row(row, &selected_fields))?;

        let result = nodes.into_iter().next().map(|node| SingleNode { node, field_names });

        Ok(result)
    }

    fn get_nodes(
        &self,
        model: ModelRef,
        query_arguments: QueryArguments,
        selected_fields: SelectedFields,
    ) -> PrismaResult<ManyNodes> {
        let scalar_fields = selected_fields.scalar_non_list();
        let field_names = scalar_fields.iter().map(|f| f.name.clone()).collect();
        let (db_name, query) = QueryBuilder::get_nodes(model, query_arguments, &selected_fields);

        let nodes = self
            .database_executor
            .with_rows(query, db_name, |row| Self::read_row(row, &selected_fields))?;

        Ok(ManyNodes { nodes, field_names })
    }

    fn get_related_nodes(
        &self,
        from_field: RelationFieldRef,
        from_node_ids: Vec<GraphqlId>,
        query_arguments: QueryArguments,
        selected_fields: SelectedFields,
    ) -> PrismaResult<ManyNodes> {
        let scalar_fields = selected_fields.scalar_non_list();
        let field_names = scalar_fields.iter().map(|f| f.name.clone()).collect();
        let (db_name, query) =
            QueryBuilder::get_related_nodes(from_field, from_node_ids, query_arguments, &selected_fields);

        let nodes = self.database_executor.with_rows(query, db_name, |row| {
            let mut node = Self::read_row(row, &selected_fields);
            let position = scalar_fields.len();

            node.add_related_id(row.get(position));
            node.add_parent_id(row.get(position + 1));
            node
        })?;

        Ok(ManyNodes { nodes, field_names })
    }

    fn count_by_model(&self, model: ModelRef, query_arguments: QueryArguments) -> PrismaResult<usize> {
        let (db_name, query) = QueryBuilder::count_by_model(model, query_arguments);

        let res = self
            .database_executor
            .with_rows(query, db_name, |row| Self::fetch_int(row))?
            .into_iter()
            .next()
            .unwrap_or(0);

        Ok(res as usize)
    }

    fn get_scalar_list_values_by_node_ids(
        &self,
        model: ModelRef,
        list_field: ScalarFieldRef,
        node_ids: Vec<GraphqlId>,
    ) -> PrismaResult<Vec<ScalarListValues>> {
        let type_identifier = list_field.type_identifier;
        let (db_name, query) = QueryBuilder::get_scalar_list_values_by_node_ids(list_field, node_ids);
        let results = self.database_executor.with_rows(query, db_name, |row| {
            let node_id: GraphqlId = row.get(0);
            let position: u32 = row.get(1);
            let value: PrismaValue = Self::fetch_value(type_identifier, row, 2);
            ScalarListElement {
                node_id,
                position,
                value,
            }
        })?;

        let mut list_values = vec![];
        for (node_id, elements) in &results.into_iter().group_by(|ele| ele.node_id.clone()) {
            let values = ScalarListValues {
                node_id,
                values: elements.into_iter().map(|e| e.value).collect(),
            };
            list_values.push(values);
        }

        Ok(list_values)
    }
}

struct ScalarListElement {
    node_id: GraphqlId,
    position: u32,
    value: PrismaValue,
}

impl<T> SqlResolver<T>
where
    T: DatabaseExecutor,
{
    pub fn new(database_executor: T) -> Self {
        Self { database_executor }
    }

    fn read_row(row: &Row, selected_fields: &SelectedFields) -> Node {
        let fields = selected_fields
            .scalar_non_list()
            .iter()
            .enumerate()
            .map(|(i, sf)| Self::fetch_value(sf.type_identifier, &row, i))
            .collect();

        Node::new(fields)
    }

    fn fetch_int(row: &Row) -> i64 {
        row.get_checked(0).unwrap_or(0)
    }

    /// Converter function to wrap the limited set of types in SQLite to a
    /// richer PrismaValue.
    fn fetch_value(typ: TypeIdentifier, row: &Row, i: usize) -> PrismaValue {
        let result = match typ {
            TypeIdentifier::String => row.get_checked(i).map(|val| PrismaValue::String(val)),
            TypeIdentifier::GraphQLID => row.get_checked(i).map(|val| PrismaValue::GraphqlId(val)),
            TypeIdentifier::UUID => row.get_checked(i).map(|val| PrismaValue::Uuid(val)),
            TypeIdentifier::Int => row.get_checked(i).map(|val| PrismaValue::Int(val)),
            TypeIdentifier::Boolean => row.get_checked(i).map(|val| PrismaValue::Boolean(val)),
            TypeIdentifier::Enum => row.get_checked(i).map(|val| PrismaValue::Enum(val)),
            TypeIdentifier::Json => row.get_checked(i).map(|val| PrismaValue::Json(val)),
            TypeIdentifier::DateTime => row.get_checked(i).map(|ts: i64| {
                let nsecs = ((ts % 1000) * 1_000_000) as u32;
                let secs = (ts / 1000) as i64;
                let naive = chrono::NaiveDateTime::from_timestamp(secs, nsecs);
                let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);

                PrismaValue::DateTime(datetime)
            }),
            TypeIdentifier::Relation => panic!("We should not have a Relation here!"),
            TypeIdentifier::Float => row.get_checked(i).map(|val: f64| PrismaValue::Float(val)),
        };

        result.unwrap_or_else(|e| match e {
            rusqlite::Error::InvalidColumnType(_, rusqlite::types::Type::Null) => PrismaValue::Null,
            _ => panic!(e),
        })
    }
}
