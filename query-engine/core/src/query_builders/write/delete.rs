use super::*;
use crate::query_builders::{Builder, ParsedField, QueryBuilderResult};
use connector::write_ast::*;
use prisma_models::ModelRef;

pub struct DeleteBuilder {
    field: ParsedField,
    model: ModelRef,
}

impl DeleteBuilder {
    pub fn new(field: ParsedField, model: ModelRef) -> Self {
        Self { field, model }
    }
}

impl Builder<WriteQuery> for DeleteBuilder {
    fn build(mut self) -> QueryBuilderResult<WriteQuery> {
        let where_arg = self.field.arguments.lookup("where").unwrap();
        let record_finder = utils::extract_record_finder(where_arg.value, &self.model)?;
        let delete = RootWriteQuery::DeleteRecord(DeleteRecord { where_: record_finder });

        Ok(WriteQuery::Root(self.field.name, self.field.alias, delete))
    }
}
