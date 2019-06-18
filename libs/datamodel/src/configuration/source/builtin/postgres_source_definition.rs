use super::{PostgresSource, POSTGRES_SOURCE_NAME};
use crate::{common::argument::Arguments, configuration::*, errors::ValidationError};

pub struct PostgresSourceDefinition {}

impl PostgresSourceDefinition {
    pub fn new() -> PostgresSourceDefinition {
        PostgresSourceDefinition {}
    }
}

impl SourceDefinition for PostgresSourceDefinition {
    fn connector_type(&self) -> &'static str {
        POSTGRES_SOURCE_NAME
    }

    fn create(
        &self,
        name: &str,
        url: &str,
        _arguments: &mut Arguments,
        documentation: &Option<String>,
    ) -> Result<Box<Source>, ValidationError> {
        Ok(Box::new(PostgresSource {
            name: String::from(name),
            url: String::from(url),
            documentation: documentation.clone(),
        }))
    }
}
