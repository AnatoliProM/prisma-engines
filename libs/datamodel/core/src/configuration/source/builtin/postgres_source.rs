use crate::{configuration::*, dml, validator::directive::DirectiveValidator};
pub const POSTGRES_SOURCE_NAME: &str = "postgresql";

pub struct PostgresSource {
    pub(super) name: String,
    pub(super) url: StringFromEnvVar,
    pub(super) documentation: Option<String>,
}

impl Source for PostgresSource {
    fn connector_type(&self) -> &str {
        POSTGRES_SOURCE_NAME
    }
    fn name(&self) -> &String {
        &self.name
    }

    fn url(&self) -> &StringFromEnvVar {
        &self.url
    }
    fn set_url(&mut self, url: &str) {
        self.url = StringFromEnvVar {
            from_env_var: None,
            value: url.to_string(),
        };
    }
    fn get_field_directives(&self) -> Vec<Box<dyn DirectiveValidator<dml::Field>>> {
        vec![]
    }
    fn get_model_directives(&self) -> Vec<Box<dyn DirectiveValidator<dml::Model>>> {
        vec![]
    }
    fn get_enum_directives(&self) -> Vec<Box<dyn DirectiveValidator<dml::Enum>>> {
        vec![]
    }
    fn documentation(&self) -> &Option<String> {
        &self.documentation
    }
}
