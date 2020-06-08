use super::database_inspection_results::DatabaseInspectionResults;

/// This trait should be implemented by warning and unexecutable migration types. It lets them
/// describe what data they need from the current state of the database to be as accurate and
/// informative as possible.
pub(super) trait Check {
    /// Fetch the row count for the table with the returned name.
    fn check_row_count(&self) -> Option<&str> {
        None
    }

    /// Fetch the number of non-null values for the returned table and column.
    fn check_existing_values(&self) -> Option<(&str, &str)> {
        None
    }

    /// This function will always be called for every check in a migration. Each change must check
    /// for the data it needs in the database inspection results. If there is no data, it should
    /// assume the current state of the database could not be inspected and warn with a best effort
    /// message explaining under which conditions the migration could not be applied or would cause
    /// data loss.
    ///
    /// The only case where `None` should be returned is when there is data about the current state
    /// of the database, and that data indicates that the migration step would be executable and
    /// safe.
    fn render(&self, database_check_results: &DatabaseInspectionResults) -> Option<String>;
}
