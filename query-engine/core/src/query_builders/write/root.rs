//! Providing an interface to build WriteQueries
#![allow(warnings)]

use crate::{
    query_builders::{
        utils, write::extend_defaults, LookAhead, ManyNestedBuilder, NestedValue, SimpleNestedBuilder,
        UpsertNestedBuilder, ValueList, ValueMap, ValueSplit, WriteQuerySet, WriteQueryTree,
    },
    schema::{ModelOperation, OperationTag},
    CoreError, CoreResult, QuerySchemaRef,
};
use connector::{filter::RecordFinder, write_ast::*};
use graphql_parser::query::{Field, Value};
use prisma_models::{Field as ModelField, InternalDataModelRef, ModelRef, PrismaArgs, PrismaValue, Project};
use std::{collections::BTreeMap, sync::Arc};

/// A root write query builder
///
/// It takes a GraphQL field and model
/// and builds a write query tree from it
#[derive(Debug)]
pub struct RootWriteQueryBuilder<'field> {
    field: &'field Field,
    query_schema: QuerySchemaRef,
}

impl<'field> RootWriteQueryBuilder<'field> {
    pub fn new(query_schema: QuerySchemaRef, field: &'field Field) -> Self {
        Self { field, query_schema }
    }

    pub fn build(self) -> CoreResult<WriteQuerySet> {
        // Handle `resetData` separately
        if &self.field.name == "resetData" {
            return handle_reset(&self.field, Arc::clone(&self.query_schema.internal_data_model));
        }

        let args = into_tree(&self.field.arguments);
        let model_operation = parse_model_action(&self.field.name, Arc::clone(&self.query_schema))?;
        let model = Arc::clone(&model_operation.model);

        let inner = match model_operation.operation {
            OperationTag::CreateOne => {
                let ValueSplit { values, lists, nested } = ValueMap(shift_data(&args, "data")?).split();
                let mut non_list_args = values.to_prisma_values();
                extend_defaults(&model, &mut non_list_args);

                let mut non_list_args: PrismaArgs = non_list_args.into();
                non_list_args.add_datetimes(Arc::clone(&model));

                let list_args = lists.into_iter().map(|la| la.convert()).collect();
                let nested_writes = build_nested_root(
                    model.name.as_str(),
                    &nested,
                    Arc::clone(&model),
                    model_operation.operation,
                )?;

                RootWriteQuery::CreateRecord(CreateRecord {
                    model: Arc::clone(&model),
                    non_list_args,
                    list_args,
                    nested_writes,
                })
            }
            OperationTag::UpdateOne => {
                let ValueSplit { values, lists, nested } = ValueMap(shift_data(&args, "data")?).split();
                let non_list_args = values.to_prisma_values().into();
                let list_args = lists.into_iter().map(|la| la.convert()).collect();
                let nested_writes = build_nested_root(
                    model.name.as_str(),
                    &nested,
                    Arc::clone(&model),
                    model_operation.operation,
                )?;

                let where_ = ValueMap(shift_data(&args, "where")?)
                    .to_record_finder(Arc::clone(&model))
                    .map_or(
                        Err(CoreError::LegacyQueryValidationError("No `where` on connect".into())),
                        |w| Ok(w),
                    )?;

                RootWriteQuery::UpdateRecord(UpdateRecord {
                    where_,
                    non_list_args,
                    list_args,
                    nested_writes,
                })
            }
            OperationTag::UpdateMany => {
                let ValueSplit { values, lists, nested } = ValueMap(shift_data(&args, "data")?).split();
                let non_list_args = values.to_prisma_values().into();
                let list_args = lists.into_iter().map(|la| la.convert()).collect();
                let nested_writes = build_nested_root(
                    model.name.as_str(),
                    &nested,
                    Arc::clone(&model),
                    model_operation.operation,
                )?;

                let query_args = utils::extract_query_args(self.field, Arc::clone(&model))?;
                let filter = query_args.filter.map(|f| Ok(f)).unwrap_or_else(|| {
                    Err(CoreError::LegacyQueryValidationError(
                        "Required filters not found!".into(),
                    ))
                })?;

                RootWriteQuery::UpdateManyRecords(UpdateManyRecords {
                    model: Arc::clone(&model),
                    filter,
                    non_list_args,
                    list_args,
                })
            }
            OperationTag::DeleteOne => {
                let where_ = ValueMap(shift_data(&args, "where")?)
                    .to_record_finder(Arc::clone(&model))
                    .map_or(
                        Err(CoreError::LegacyQueryValidationError("No `where` on connect".into())),
                        |w| Ok(w),
                    )?;

                RootWriteQuery::DeleteRecord(DeleteRecord { where_ })
            }
            OperationTag::DeleteMany => {
                let query_args = utils::extract_query_args(self.field, Arc::clone(&model))?;
                let filter = query_args.filter.map(|f| Ok(f)).unwrap_or_else(|| {
                    Err(CoreError::LegacyQueryValidationError(
                        "Required filters not found!".into(),
                    ))
                })?;

                RootWriteQuery::DeleteManyRecords(DeleteManyRecords { model, filter })
            }
            OperationTag::UpsertOne => {
                let where_ = utils::extract_record_finder(self.field, Arc::clone(&model))?;

                let create = {
                    let ValueSplit { values, lists, nested } = ValueMap(shift_data(&args, "create")?).split();
                    let mut non_list_args = values.to_prisma_values();
                    extend_defaults(&model, &mut non_list_args);

                    let mut non_list_args: PrismaArgs = non_list_args.into();
                    non_list_args.add_datetimes(Arc::clone(&model));

                    let list_args = lists.into_iter().map(|la| la.convert()).collect();
                    let nested_writes = build_nested_root(
                        model.name.as_str(),
                        &nested,
                        Arc::clone(&model),
                        model_operation.operation,
                    )?;
                    let model = Arc::clone(&model);

                    CreateRecord {
                        model,
                        non_list_args,
                        list_args,
                        nested_writes: nested_writes,
                    }
                };

                let update = {
                    let ValueSplit { values, lists, nested } = ValueMap(shift_data(&args, "update")?).split();
                    let non_list_args = values.to_prisma_values().into();
                    let list_args = lists.into_iter().map(|la| la.convert()).collect();
                    let nested_writes = build_nested_root(
                        model.name.as_str(),
                        &nested,
                        Arc::clone(&model),
                        model_operation.operation,
                    )?;
                    let where_ = where_.clone();

                    UpdateRecord {
                        where_,
                        non_list_args,
                        list_args,
                        nested_writes,
                    }
                };

                RootWriteQuery::UpsertRecord(UpsertRecord { where_, create, update })
            }
            _ => unimplemented!(),
        };

        // FIXME: Cloning is unethical and should be avoided
        LookAhead::eval(WriteQueryTree {
            inner,
            name: self.field.alias.as_ref().unwrap_or_else(|| &self.field.name).clone(),
            field: self.field.clone(),
        })
    }
}

/// A trap-door function that handles `resetData` without doing a whole bunch of other stuff
fn handle_reset(field: &Field, internal_data_model: InternalDataModelRef) -> CoreResult<WriteQuerySet> {
    Ok(WriteQuerySet::Query(WriteQueryTree {
        inner: RootWriteQuery::ResetData(ResetData { internal_data_model }),
        name: "resetData".into(),
        field: field.clone(),
    }))
}

/// Convert arguments provided by graphql-ast into a tree
fn into_tree(from: &Vec<(String, Value)>) -> BTreeMap<String, Value> {
    from.into_iter().map(|(a, b)| (a.clone(), b.clone())).collect()
}

/// Shift into a sub-tree of arguments
fn shift_data(from: &BTreeMap<String, Value>, idx: &str) -> CoreResult<BTreeMap<String, Value>> {
    from.get(idx).map_or(
        Err(CoreError::LegacyQueryValidationError(format!(
            "Failed to resolve `{}` block!",
            idx
        ))),
        |c| match c {
            Value::Object(obj) => Ok(obj.clone()),
            child => Err(CoreError::LegacyQueryValidationError(format!(
                "Invalid child type for `{}`: `{}`",
                idx, child
            ))),
        },
    )
}

/// Parse the field name into an operation to perform.
fn parse_model_action(name: &str, query_schema: QuerySchemaRef) -> CoreResult<ModelOperation> {
    match query_schema.find_mutation_field(name) {
        Some(field) => Ok(field
            .operation
            .clone()
            .expect("Expected top level field to have an associated model operation.")),

        None => Err(CoreError::LegacyQueryValidationError(format!(
            "Field not found on type Mutation: {}",
            name
        ))),
    }
}

/// Build nested writes for a given field/model (called recursively)
pub(crate) fn build_nested_root<'f>(
    name: &'f str,
    args: &'f ValueMap,
    model: ModelRef,
    top_level: OperationTag,
) -> CoreResult<NestedWriteQueries> {
    let mut collection = NestedWriteQueries::default();
    let eval = args.eval_tree(model.name.as_str());

    for value in eval.into_iter() {
        match value {
            NestedValue::Simple { name, kind, map } => {
                SimpleNestedBuilder::build(name, kind, map, &mut collection, Arc::clone(&model), None, top_level)?
            }
            NestedValue::Block {
                name,
                kind,
                data,
                where_,
            } => SimpleNestedBuilder::build(
                name,
                kind,
                data,
                &mut collection,
                Arc::clone(&model),
                Some(where_),
                top_level,
            )?,
            NestedValue::Many { name, kind, list } => ManyNestedBuilder::build(
                name,
                kind,
                list.into_iter(),
                &mut collection,
                Arc::clone(&model),
                top_level,
            )?,
            NestedValue::Upsert {
                name,
                create,
                update,
                where_,
            } => UpsertNestedBuilder::build(
                name,
                where_,
                create,
                update,
                &mut collection,
                Arc::clone(&model),
                top_level,
            )?,
            _ => unimplemented!(),
        };
    }

    Ok(collection)
}
