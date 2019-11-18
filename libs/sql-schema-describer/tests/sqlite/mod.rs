use log::debug;
use sql_connection::{GenericSqlConnection, SqlConnection};
use sql_schema_describer::*;
use std::path::Path;
use std::sync::Arc;

use super::SCHEMA;

pub async fn get_sqlite_describer(sql: &str) -> sqlite::SqlSchemaDescriber {
    let server_root = std::env::var("SERVER_ROOT").expect("Env var SERVER_ROOT required but not found.");
    let database_folder_path = format!("{}/db", server_root);
    let database_file_path = format!("{}/{}.db", database_folder_path, SCHEMA);
    debug!("Database file path: '{}'", database_file_path);

    if Path::new(&database_file_path).exists() {
        std::fs::remove_file(database_file_path.clone()).expect("remove database file");
    }

    let conn = GenericSqlConnection::from_database_str(&format!("file://{}", database_file_path), Some(SCHEMA)).unwrap();
    for statement in sql.split(";").filter(|statement| !statement.is_empty()) {
        conn.execute_raw(statement, &[]).await.expect("executing migration");
    }

    sqlite::SqlSchemaDescriber::new(Arc::new(conn))
}
