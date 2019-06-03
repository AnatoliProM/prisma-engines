use crate::common::ErrorAsserts;
use datamodel::errors::ValidationError;

const DATAMODEL: &str = r#"
source pg1 {
    name = "Postgres"
    url = "https://localhost/postgres1"
}

source pg2 {
    name = "Postgres"
    url = "https://localhost/postgres2"
}

source sqlite1 {
    name = "Sqlite"
    url = "https://localhost/sqlite1"
}

source mysql1 {
    name = "MySQL"
    url = "https://localhost/mysql"
}
"#;

#[test]
fn serialize_builtin_sources_to_dmmf() {
    let sources = datamodel::load_data_source_configuration(DATAMODEL).unwrap();
    let rendered = datamodel::dmmf::render_config_to_dmmf(&sources);

    let expected = r#"[
  {
    "name": "pg1",
    "connectorName": "Postgres",
    "url": "https://localhost/postgres1",
    "config": {}
  },
  {
    "name": "pg2",
    "connectorName": "Postgres",
    "url": "https://localhost/postgres2",
    "config": {}
  },
  {
    "name": "sqlite1",
    "connectorName": "Sqlite",
    "url": "https://localhost/sqlite1",
    "config": {}
  },
  {
    "name": "mysql1",
    "connectorName": "MySQL",
    "url": "https://localhost/mysql",
    "config": {}
  }
]"#;

    assert_eq!(rendered, expected);
}

const INVALID_DATAMODEL: &str = r#"
source pg1 {
    name = "AStrangeHalfMongoDatabase"
    url = "https://localhost/postgres1"
}
"#;

#[test]
fn fail_to_load_sources_for_invalid_source() {
    let res = datamodel::load_data_source_configuration(INVALID_DATAMODEL);

    if let Err(error) = res {
        error.assert_is(ValidationError::SourceNotKnownError {
            source_name: String::from("pg1"),
            span: datamodel::ast::Span::new(1, 94),
        });
    } else {
        panic!("Expected error.")
    }
}
