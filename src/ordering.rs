use crate::{error::Error, models::prelude::*, protobuf::prelude::*, PrismaResult};
use sql::prelude::*;
use std::sync::Arc;

pub type OrderVec = Vec<(Column, Option<Order>)>;

pub struct Ordering;

impl Ordering {
    pub fn for_model(model: &Model, args: &QueryArguments) -> PrismaResult<OrderVec> {
        let first = match args.order_by {
            Some(ref order) => Some(model.fields().find_from_scalar(&order.scalar_field)?),
            None => None,
        };

        Self::by_fields(first, model.fields().id(), args)
    }

    fn by_fields(
        first_field: Option<Arc<ScalarField>>,
        second_field: Arc<ScalarField>,
        args: &QueryArguments,
    ) -> PrismaResult<OrderVec> {
        match (args.first, args.last) {
            (Some(_), Some(_)) => Err(Error::InvalidConnectionArguments(
                "Cannot have first and last both set in a select",
            )),
            (_, last) => {
                let is_reverse_order = last.is_some();

                let default_order = args
                    .order_by
                    .as_ref()
                    .map(|order| order.sort_order())
                    .unwrap_or(SortOrder::Asc);

                let ordering = match first_field {
                    Some(ref first) if first.db_name() != second_field.db_name() => {
                        match (default_order, is_reverse_order) {
                            (SortOrder::Asc, true) => vec![
                                first.model_column().descend(),
                                second_field.model_column().descend(),
                            ],
                            (SortOrder::Desc, true) => vec![
                                first.model_column().ascend(),
                                second_field.model_column().descend(),
                            ],
                            (SortOrder::Asc, false) => vec![
                                first.model_column().ascend(),
                                second_field.model_column().descend(),
                            ],
                            (SortOrder::Desc, false) => vec![
                                first.model_column().descend(),
                                second_field.model_column().ascend(),
                            ],
                        }
                    }
                    _ => match (default_order, is_reverse_order) {
                        (SortOrder::Asc, true) => vec![second_field.model_column().descend()],
                        (SortOrder::Desc, true) => vec![second_field.model_column().ascend()],
                        (SortOrder::Asc, false) => vec![second_field.model_column().ascend()],
                        (SortOrder::Desc, false) => vec![second_field.model_column().descend()],
                    },
                };

                Ok(ordering)
            }
        }
    }
}
