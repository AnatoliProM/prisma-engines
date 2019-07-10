use super::*;
use crate::query_builders::{ParsedArgument, ParsedInputValue, QueryBuilderResult};
use connector::{filter::RecordFinder, QueryArguments};
use prisma_models::ModelRef;
use std::{collections::BTreeMap, convert::TryInto};

/// Expects the caller to know that it is structurally guaranteed that a record finder can be extracted
/// from the given set of arguments, e.g. that the query schema guarantees that the necessary fields are present.
/// Errors occur if the arguments are structurally correct, but it's semantically impossible
/// to extract a record finder, e.g. if too many fields are given.
pub fn extract_record_finder(arguments: Vec<ParsedArgument>, model: &ModelRef) -> QueryBuilderResult<RecordFinder> {
    let where_arg = arguments.into_iter().find(|arg| arg.name == "where").unwrap();
    let values: BTreeMap<String, ParsedInputValue> = where_arg.value.try_into().unwrap();

    if values.len() != 1 {
        Err(QueryValidationError::AssertionError(format!(
            "Expected exactly one value for 'where' argument, got: {}",
            values.iter().map(|v| v.0.as_str()).collect::<Vec<&str>>().join(", ")
        )))
    } else {
        let field_selector: (String, ParsedInputValue) = values.into_iter().next().unwrap();
        let model_field = model.fields().find_from_scalar(&field_selector.0).unwrap();

        Ok(RecordFinder {
            field: model_field,
            value: field_selector.1.try_into().unwrap(),
        })
    }
}

/// Expects the caller to know that it is structurally guaranteed that query arguments can be extracted,
/// e.g. that the query schema guarantees that required fields are present.
/// Errors occur if ...
/// Umwraps are safe because the schema validation guarantees type conformity.
pub fn extract_query_args(arguments: Vec<ParsedArgument>) -> QueryBuilderResult<QueryArguments> {
    arguments
        .into_iter()
        .fold(Ok(QueryArguments::default()), |result, arg| {
            if let Ok(res) = result {
                dbg!(&res);
                dbg!(&arg);
                match arg.name.as_str() {
                    "skip" => Ok(QueryArguments {
                        skip: arg.value.try_into().unwrap(),
                        ..res
                    }),
                    "first" => Ok(QueryArguments {
                        first: arg.value.try_into().unwrap(),
                        ..res
                    }),
                    "last" => Ok(QueryArguments {
                        last: arg.value.try_into().unwrap(),
                        ..res
                    }),
                    "after" => Ok(QueryArguments {
                        after: arg.value.try_into().unwrap(),
                        ..res
                    }),
                    "before" => Ok(QueryArguments {
                        before: arg.value.try_into().unwrap(),
                        ..res
                    }),
                    "orderby" => Ok(QueryArguments {
                        order_by: arg.value.try_into().unwrap(),
                        ..res
                    }),
                    "where" => Ok(QueryArguments {
                        order_by: arg.value.try_into().unwrap(),
                        ..res
                    }),
                    _ => Ok(res),
                }
            } else {
                result
            }
        })
}
