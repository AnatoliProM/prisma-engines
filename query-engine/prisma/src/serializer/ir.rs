//! Serializer Intermediate Representation
//!
//! Flexible intermediate representation for `PrismaQueryResult`s
//! which associates data from subsequent chained and nested queries
//! correctly.
//!
//! In the main `PrismaQueraResult` DSL, there's no trivial way of
//! associating data from a nested multi-query with a parent.
//! This IR fixes that issue, allowing us to serialize to various
//! flexible formats.

use core::{MultiPrismaQueryResult, PrismaQueryResult, SinglePrismaQueryResult};
use indexmap::IndexMap;
use itertools::{
    EitherOrBoth::{Both, Left, Right},
    Itertools,
};
use prisma_models::PrismaValue;

/// A set of responses to provided queries
pub type Responses = Vec<IrResponse>;

#[allow(dead_code)]
pub enum IrResponse {
    Data(String, Item),
    Error(String), // TODO: Get a better error kind?
}

/// A key -> value map to an IR item
pub type Map = IndexMap<String, Item>;

/// A list of IR items
pub type List = Vec<Item>;

/// An IR item that either expands to a subtype or leaf-node
#[derive(Debug)]
pub enum Item {
    Map(Map),
    List(List),
    Value(PrismaValue),
}

/// A serialization IR builder utility
pub struct IrBuilder<'results>(Vec<&'results PrismaQueryResult>);

impl<'results> IrBuilder<'results> {
    pub fn new() -> Self {
        Self(vec![])
    }

    /// Add a single query result to the builder
    pub fn add(mut self, q: &'results PrismaQueryResult) -> Self {
        self.0.push(q);
        self
    }

    /// Parse collected queries into a wrapper type
    pub fn build(self) -> Responses {
        self.0.into_iter().fold(vec![], |mut vec, res| {
            vec.push(match res {
                PrismaQueryResult::Single(query) => IrResponse::Data(query.name.clone(), Item::Map(build_map(query))),
                PrismaQueryResult::Multi(query) => IrResponse::Data(query.name.clone(), Item::List(build_list(query))),
            });
            vec
        })
    }
}

fn build_map(result: &SinglePrismaQueryResult) -> Map {
    // Build selected fields first
    let mut outer = match &result.result {
        Some(single) => single
            .field_names
            .iter()
            .zip(&single.node.values)
            .fold(Map::new(), |mut map, (name, val)| {
                map.insert(name.clone(), Item::Value(val.clone()));
                map
            }),
        None => panic!("No result found"), // FIXME: Can this ever happen?
    };

    // Then add nested selected fields
    outer = result.nested.iter().fold(outer, |mut map, query| {
        match query {
            PrismaQueryResult::Single(nested) => map.insert(nested.name.clone(), Item::Map(build_map(nested))),
            PrismaQueryResult::Multi(nested) => map.insert(nested.name.clone(), Item::List(build_list(nested))),
        };

        map
    });

    result
        .list_results
        .values
        .iter()
        .zip(&result.list_results.field_names)
        .for_each(|(values, field_name)| {
            outer.insert(
                field_name.clone(),
                Item::List(values.iter().fold(vec![], |_, list| {
                    list.iter().map(|pv| Item::Value(pv.clone())).collect()
                })),
            );
        });

    result.fields.iter().fold(Map::new(), |mut map, field| {
        map.insert(
            field.clone(),
            outer.remove(field).expect("[Map]: Missing required field"),
        );
        map
    })
}

fn build_list(result: &MultiPrismaQueryResult) -> List {
    let mut vec: Vec<Item> = result
        .result
        .as_pairs()
        .iter()
        .map(|vec| {
            Item::Map(vec.iter().fold(Map::new(), |mut map, (name, value)| {
                map.insert(name.clone(), Item::Value(value.clone()));
                map
            }))
        })
        .collect();

    result.nested.iter().zip(&mut vec).for_each(|(nested, map)| {
        match map {
            Item::Map(ref mut map) => match nested {
                PrismaQueryResult::Single(nested) => map.insert(nested.name.clone(), Item::Map(build_map(nested))),
                PrismaQueryResult::Multi(nested) => map.insert(nested.name.clone(), Item::List(build_list(nested))),
            },
            _ => unreachable!(),
        };
    });

    // Associate scalarlists with corresponding records
    //
    // This is done by iterating through both existing records and the list of
    // list-results. But because the list-results list can be shorter, we need
    // to zip_longest() which yields a special enum. We differentiate between
    // "only record was found" and "both record and corresponding list data
    // was found". In case there's only list data but no record, we panic.
    vec.iter_mut().zip_longest(&result.list_results.values).for_each(|eob| {
        match eob {

            Both(item, values) => {
                if let Item::Map(ref mut map) = item {
                    values
                        .iter()
                        .zip(&result.list_results.field_names)
                        .for_each(|(list, field_name)| {
                            map.insert(
                                field_name.clone(),
                                Item::List(list.iter().map(|pv| Item::Value(pv.clone())).collect()),
                            );
                        })
                }
            }
            Left(item) => {
                if let Item::Map(ref mut map) = item {
                    result.list_results.field_names.iter().for_each(|field_name| {
                        map.insert(field_name.clone(), Item::List(vec![]));
                    })
                }
            }
            _ => unreachable!("Unexpected scalar-list values for missing record")
        };
    });


    vec.into_iter()
        .fold(vec![], |mut vec, mut item| {
            if let Item::Map(ref mut map) = item {
                vec.push(result.fields.iter().fold(Map::new(), |mut new, field| {
                    let item = map.remove(field).expect("[List]: Missing required field");
                    new.insert(field.clone(), item);
                    new
                }));
            }

            vec
        })
        .into_iter()
        .map(|i| Item::Map(i))
        .collect()
}
