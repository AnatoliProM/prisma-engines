use crate::{dml, dml::validator::directive::DirectiveValidator, source::*};
pub const POSTGRES_SOURCE_NAME: &str = "Postgres";

pub struct PostgresSource {
    pub(super) name: String,
    pub(super) url: String,
}

impl Source for PostgresSource {
    fn connector_name(&self) -> &str {
        POSTGRES_SOURCE_NAME
    }
    fn name(&self) -> &String {
        &self.name
    }
    fn config(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::new()
    }
    fn url(&self) -> &String {
        &self.url
    }
    fn get_field_directives(&self) -> Vec<Box<DirectiveValidator<dml::Field>>> {
        vec![]
    }
    fn get_model_directives(&self) -> Vec<Box<DirectiveValidator<dml::Model>>> {
        vec![]
    }
    fn get_enum_directives(&self) -> Vec<Box<DirectiveValidator<dml::Enum>>> {
        vec![]
    }
}
