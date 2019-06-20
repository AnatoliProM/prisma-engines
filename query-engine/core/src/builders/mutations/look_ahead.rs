//! # ✨ Crunchtime hacks ✨
//!
//! This module evaluates a nested mutation and merges it's operations
//! into the current-level if it deems it neccessary.
//! This is to get around issues with required references
//! that are connected in a second step.

use crate::{CoreResult, WriteQuery, WriteQuerySet};
use connector::mutaction::*;
use graphql_parser::query::Field;
use prisma_models::{ModelRef, PrismaArgs, PrismaValue};

pub struct LookAhead;
impl LookAhead {
    pub fn eval(mut input: WriteQuery) -> CoreResult<WriteQuerySet> {
        input.inner = match input.inner {
            TopLevelDatabaseMutaction::CreateRecord(mut cn) => {
                create_connect(&mut cn)?;
                TopLevelDatabaseMutaction::CreateRecord(cn)
            }
            TopLevelDatabaseMutaction::UpdateRecord(mut un) => {
                update_nested_connect(&mut un)?;
                TopLevelDatabaseMutaction::UpdateRecord(un)
            }
            who_even_cares => who_even_cares,
        };

        let flipped = flip_create_order(input);
        debug!("{:#?}", flipped);
        flipped
    }

    /// This function is called in the QueryExecutor, after executing a partial mutation tree
    ///
    /// What it needs to do is work with the result of the partial execution,
    /// then inject any IDs or data into the base mutation of the Dependents tree
    pub fn eval_partial(next: &mut WriteQuerySet, self_: &WriteQuery, res: &DatabaseMutactionResult) -> CoreResult<()> {
        let name = next
            .get_base_model()
            .fields()
            .find_from_relation_fields(&self_.model().name.to_lowercase())?
            .name
            .clone();
        let id: PrismaValue = match res.identifier {
            Identifier::Id(ref gqlid) => gqlid.into(),
            _ => unimplemented!(),
        };

        next.inject_at_base(move |query| match query.inner {
            TopLevelDatabaseMutaction::CreateRecord(ref mut cn) => cn.non_list_args.insert(name, id),
            _ => unimplemented!(),
        });

        Ok(())
    }
}

/// Merge connect's on required fields into the create
fn create_connect(cn: &mut CreateRecord) -> CoreResult<()> {
    let connects = std::mem::replace(&mut cn.nested_mutactions.connects, vec![]);

    let mut new = vec![];
    connect_fold_into(&cn.model, &mut cn.non_list_args, &mut new, connects.into_iter())?;
    cn.nested_mutactions.connects = new;

    // Now recursively traverse all other creates
    for ncn in cn.nested_mutactions.creates.iter_mut() {
        nested_create_connect(ncn)?;
    }

    Ok(())
}

fn nested_create_connect(ncn: &mut NestedCreateRecord) -> CoreResult<()> {
    let connects = std::mem::replace(&mut ncn.nested_mutactions.connects, vec![]);
    let mut new = vec![];

    connect_fold_into(
        &ncn.relation_field.model(),
        &mut ncn.non_list_args,
        &mut new,
        connects.into_iter(),
    )?;

    ncn.nested_mutactions.connects = new;

    for cn in ncn.nested_mutactions.creates.iter_mut() {
        nested_create_connect(cn)?;
    }

    Ok(())
}

fn update_nested_connect(un: &mut UpdateRecord) -> CoreResult<()> {
    for ncn in un.nested_mutactions.creates.iter_mut() {
        nested_create_connect(ncn)?;
    }

    Ok(())
}

fn flip_create_order(wq: WriteQuery) -> CoreResult<WriteQuerySet> {
    match wq.inner {
        TopLevelDatabaseMutaction::CreateRecord(mut cn) => {
            let creates = std::mem::replace(&mut cn.nested_mutactions.creates, vec![]);
            let (required, normal) = creates.into_iter().partition(|nc| {
                nc.relation_field.is_required
                    && check_should_flip(&cn.model, &nc.relation_field.related_field().model())
            });

            cn.nested_mutactions.creates = normal;

            let wq = WriteQuery {
                inner: TopLevelDatabaseMutaction::CreateRecord(cn),
                ..wq
            };

            let wqs: WriteQuerySet = required
                .into_iter()
                .fold(WriteQuerySet::Query(wq), |acc, req| match acc {
                    WriteQuerySet::Query(q) => WriteQuerySet::Dependents {
                        self_: WriteQuery {
                            inner: hoist_nested_create(req),
                            name: q.name.clone(),
                            field: q.field.clone(),
                        },
                        next: Box::new(WriteQuerySet::Query(q)),
                    },
                    WriteQuerySet::Dependents { self_: _, ref next } => {
                        let (name, field) = get_name_field(&next);

                        WriteQuerySet::Dependents {
                            self_: WriteQuery {
                                inner: hoist_nested_create(req),
                                name,
                                field,
                            },
                            next: Box::new(acc),
                        }
                    }
                });

            Ok(wqs)
        }
        _ => Ok(WriteQuerySet::Query(wq)),
    }
}

/// Takes a NestedCreateRecord and turns it into a TopLevelDatabaseMutaction::CreateRecord
fn hoist_nested_create(nc: NestedCreateRecord) -> TopLevelDatabaseMutaction {
    TopLevelDatabaseMutaction::CreateRecord(CreateRecord {
        model: nc.relation_field.related_field().model(),
        non_list_args: nc.non_list_args,
        list_args: nc.list_args,
        nested_mutactions: nc.nested_mutactions,
    })
}

/// Small utility function to get a query name and field
fn get_name_field(next: &WriteQuerySet) -> (String, Field) {
    match next {
        WriteQuerySet::Dependents { self_: _, next } => get_name_field(&next),
        WriteQuerySet::Query(q) => (q.name.clone(), q.field.clone()),
    }
}

fn check_should_flip(self_: &ModelRef, other: &ModelRef) -> bool {
    self_.name > other.name
}

/// Fold require `connect` operations into their parent
///
/// This function requires the model definition of the parent,
/// an argument set to fold into, a vector for non-required operations
/// and a set of connects to evaluate.
fn connect_fold_into(
    model: &ModelRef,
    args: &mut PrismaArgs,
    nested: &mut Vec<NestedConnect>,
    connects: impl Iterator<Item = NestedConnect>,
) -> CoreResult<()> {
    for conn in connects {
        let rf = model
            .fields()
            .find_from_relation_fields(conn.relation_field.name.as_str())?;

        if rf.is_required {
            args.insert(rf.name.clone(), conn.where_.value);
        } else {
            nested.push(conn);
        }
    }

    Ok(())
}
