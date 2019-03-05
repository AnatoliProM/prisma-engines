use crate::{
    cursor_condition::CursorCondition,
    data_resolvers::{IntoSelectQuery, SelectQuery},
    ordering::Ordering,
    protobuf::filter::IntoFilter,
    protobuf::prelude::*,
};

use prisma_common::PrismaResult;
use prisma_models::prelude::*;
use prisma_query::ast::*;

impl IntoSelectQuery for GetNodesInput {
    fn into_select_query(self) -> PrismaResult<SelectQuery> {
        let project_template: ProjectTemplate = serde_json::from_reader(self.project_json.as_slice())?;

        let project: ProjectRef = project_template.into();
        let model = project.schema().find_model(&self.model_name)?;
        let selected_fields = Self::selected_fields(&model, self.selected_fields);

        let ordering = Ordering::for_model(
            &model,
            &self.query_arguments.order_by,
            self.query_arguments.last.is_some(),
        )?;

        let cursor: ConditionTree = CursorCondition::build(&self.query_arguments, &model);

        let filter: ConditionTree = dbg!(self.query_arguments.filter)
            .map(|filter| filter.into_filter(model.clone()))
            .map(|filter| filter.into())
            .unwrap_or(ConditionTree::NoCondition);

        let conditions = ConditionTree::and(filter, cursor);

        let (skip, limit) = match self.query_arguments.last.or(self.query_arguments.first) {
            Some(c) => (self.query_arguments.skip.unwrap_or(0), Some(c + 1)), // +1 to see if there's more data
            None => (self.query_arguments.skip.unwrap_or(0), None),
        };

        let base_query = Self::base_query(model.db_name(), conditions, skip as usize);
        let with_columns = Self::select_fields(base_query, &selected_fields);
        let ordered = Self::order_by(with_columns, ordering);
        let select_ast = Self::limit(ordered, limit.map(|limit| limit as usize));

        dbg!(Ok(SelectQuery {
            db_name: project.schema().db_name.to_string(),
            query_ast: select_ast,
            selected_fields: selected_fields,
        }))
    }
}
