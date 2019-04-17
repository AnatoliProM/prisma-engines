use connector::ScalarListValues;
use prisma_models::{GraphqlId, ManyNodes, PrismaValue, SelectedFields, SelectedScalarField, SingleNode};

#[derive(Debug)]
pub enum ReadQueryResult {
    Single(SingleReadQueryResult),
    Many(ManyReadQueryResults),
}

#[derive(Debug)]
pub struct SingleReadQueryResult {
    pub name: String,
    pub fields: Vec<String>,
    pub result: Option<SingleNode>,
    pub nested: Vec<ReadQueryResult>,

    /// Scalar list field names mapped to their results
    pub list_results: Vec<(String, Vec<ScalarListValues>)>,

    /// Used for filtering implicit fields in result records
    pub selected_fields: SelectedFields,
}

#[derive(Debug)]
pub struct ManyReadQueryResults {
    pub name: String,
    pub fields: Vec<String>,
    pub result: ManyNodes,
    pub nested: Vec<ReadQueryResult>,

    /// Scalar list field names mapped to their results
    pub list_results: Vec<(String, Vec<ScalarListValues>)>,

    /// Used for filtering implicit fields in result records
    pub selected_fields: SelectedFields,
}

// Q: Best pattern here? Mix of in place mutation and recreating result
impl SingleReadQueryResult {
    /// Returns the implicitly added fields
    pub fn get_implicit_fields(&self) -> Vec<&SelectedScalarField> {
        self.selected_fields.get_implicit_fields()
    }

    /// Get the ID from a record
    pub fn find_id(&self) -> Option<&GraphqlId> {
        let id_position: usize = self
            .result
            .as_ref()
            .map_or(None, |r| r.field_names.iter().position(|name| name == "id"))?;

        self.result.as_ref().map_or(None, |r| {
            r.node.values.get(id_position).map(|pv| match pv {
                PrismaValue::GraphqlId(id) => Some(id),
                _ => None,
            })?
        })
    }
}

impl ManyReadQueryResults {
    /// Returns the implicitly added fields
    pub fn get_implicit_fields(&self) -> Vec<&SelectedScalarField> {
        self.selected_fields.get_implicit_fields()
    }

    /// Get all IDs from a query result
    pub fn find_ids(&self) -> Option<Vec<&GraphqlId>> {
        let id_position: usize = self.result.field_names.iter().position(|name| name == "id")?;
        self.result
            .nodes
            .iter()
            .map(|node| node.values.get(id_position))
            .map(|pv| match pv {
                Some(PrismaValue::GraphqlId(id)) => Some(id),
                _ => None,
            })
            .collect()
    }
}
