use crate::{ConnectorResult, RootWriteQuery, WriteQueryResult};
use serde_json::Value;

/// Methods for writing data.
pub trait UnmanagedDatabaseWriter {
    /// Execute raw SQL string without any safety guarantees, returning the result as JSON.
    fn execute_raw(&self, db_name: String, query: String) -> ConnectorResult<Value>;

    /// Executes the write query and all nested write queries, returning the result
    /// of the topmost write.
    fn execute(&self, db_name: String, write_query: RootWriteQuery) -> ConnectorResult<WriteQueryResult>;
}
