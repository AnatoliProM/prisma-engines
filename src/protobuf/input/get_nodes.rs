use crate::{
    connectors::{IntoSelectQuery, SelectQuery},
    models::prelude::*,
    protobuf::prelude::*,
    PrismaResult,
};

use sql::prelude::*;
use std::collections::BTreeSet;

impl IntoSelectQuery for GetNodesInput {
    fn into_select_query(self) -> PrismaResult<SelectQuery> {
        let project_template: ProjectTemplate =
            serde_json::from_reader(self.project_json.as_slice())?;

        let fields = self
            .selected_fields
            .into_iter()
            .fold(BTreeSet::new(), |mut acc, field| {
                if let Some(selected_field::Field::Scalar(s)) = field.field {
                    acc.insert(s);
                };
                acc
            });

        let filter = self
            .query_arguments
            .filter
            .map(|filter| filter.into())
            .unwrap_or(ConditionTree::NoCondition);

        let query = SelectQuery {
            project: project_template.into(),
            model_name: self.model_name,
            selected_fields: fields,
            conditions: filter,
            order_by: None, // TODO
            skip: self.query_arguments.skip,
            after: self.query_arguments.after,
            first: self.query_arguments.first,
        };

        Ok(query)
    }
}
