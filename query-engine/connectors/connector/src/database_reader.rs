use crate::{filter::RecordFinder, query_arguments::QueryArguments, ConnectorResult};
use prisma_models::prelude::*;
use prisma_models::ScalarFieldRef;

/// Methods for fetching data.
pub trait DatabaseReader {
    /// Find one record.
    fn get_single_record(
        &self,
        record_finder: &RecordFinder,
        selected_fields: &SelectedFields,
    ) -> ConnectorResult<Option<SingleRecord>>;

    /// Filter many records.
    fn get_many_records(
        &self,
        model: ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &SelectedFields,
    ) -> ConnectorResult<ManyRecords>;

    /// Filter records related to the parent.
    fn get_related_records(
        &self,
        from_field: RelationFieldRef,
        from_record_ids: &[GraphqlId],
        query_arguments: QueryArguments,
        selected_fields: &SelectedFields,
    ) -> ConnectorResult<ManyRecords>;

    /// Fetch scalar list values for the parent.
    fn get_scalar_list_values_by_record_ids(
        &self,
        list_field: ScalarFieldRef,
        record_ids: Vec<GraphqlId>,
    ) -> ConnectorResult<Vec<ScalarListValues>>;

    /// Count the items in the model with the given arguments.
    fn count_by_model(&self, model: ModelRef, query_arguments: QueryArguments) -> ConnectorResult<usize>;

    /// Count the items in the table.
    fn count_by_table(&self, database: &str, table: &str) -> ConnectorResult<usize>;
}

#[derive(Debug)]
pub struct ScalarListValues {
    pub record_id: GraphqlId,
    pub values: Vec<PrismaValue>,
}
