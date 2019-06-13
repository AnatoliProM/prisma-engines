mod database_schema_calculator;
mod database_schema_differ;
mod sql_database_migration_inferrer;
mod sql_database_step_applier;
mod sql_destructive_changes_checker;
mod sql_migration;
mod sql_migration_persistence;

use database_inspector::DatabaseInspector;
use migration_connector::*;
use postgres::Config as PostgresConfig;
use prisma_query::connector::{PostgreSql, Sqlite};
use prisma_query::Connectional;
use serde_json;
use sql_database_migration_inferrer::*;
use sql_database_step_applier::*;
use sql_destructive_changes_checker::*;
pub use sql_migration::*;
use sql_migration_persistence::*;
use std::convert::TryFrom;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use url::Url;
use std::borrow::Cow;

#[allow(unused, dead_code)]
pub struct SqlMigrationConnector {
    pub file_path: Option<String>,
    pub sql_family: SqlFamily,
    pub schema_name: String,
    pub connectional: Arc<Connectional>,
    pub migration_persistence: Arc<MigrationPersistence>,
    pub database_migration_inferrer: Arc<DatabaseMigrationInferrer<SqlMigration>>,
    pub database_migration_step_applier: Arc<DatabaseMigrationStepApplier<SqlMigration>>,
    pub destructive_changes_checker: Arc<DestructiveChangesChecker<SqlMigration>>,
    pub database_inspector: Arc<DatabaseInspector>,
}

#[derive(Copy, Clone, PartialEq)]
pub enum SqlFamily {
    Sqlite,
    Postgres,
    Mysql,
}

impl SqlMigrationConnector {
    #[allow(unused)]
    pub fn exists(sql_family: SqlFamily, url: &str) -> bool {
        match sql_family {
            SqlFamily::Sqlite => {
                let sqlite = Sqlite::try_from(url).expect("Loading SQLite failed");
                sqlite.does_file_exist()
            }
            SqlFamily::Postgres => {
                let postgres_helper = Self::postgres_helper(&url);
                let check_sql = format!("SELECT schema_name FROM information_schema.schemata WHERE schema_name = '{}';", postgres_helper.schema);
                let result_set = postgres_helper.db_connection.query_on_raw_connection("", &check_sql, &[]);
                result_set.into_iter().next().is_some()
            },
            _ => unimplemented!(),
        }
    }

    pub fn new(sql_family: SqlFamily, url: &str) -> Arc<MigrationConnector<DatabaseMigration = SqlMigration>> {
        match sql_family {
            SqlFamily::Sqlite => {
                assert!(url.starts_with("file:"), "the url for sqlite must start with 'file:'");
                let conn = Arc::new(Sqlite::try_from(url).expect("Loading SQLite failed"));
                let schema_name = "lift".to_string();
                let file_path = url.trim_start_matches("file:").to_string();
                Self::create_connector(conn, sql_family, schema_name, Some(file_path))
            }
            SqlFamily::Postgres => {                
                let postgres_helper = Self::postgres_helper(&url);
                Self::create_connector(postgres_helper.db_connection, sql_family, postgres_helper.schema, None)
            }
            _ => unimplemented!(),
        }
    }

    fn postgres_helper(url: &str) -> PostgresHelper {
        let connection_limit = 10;
        let parsed_url = Url::parse(url).expect("Parsing of the provided connector url failed.");
        let mut config = PostgresConfig::new();
        if let Some(host) = parsed_url.host_str() {
            config.host(host);
        }
        config.user(parsed_url.username());
        if let Some(password) = parsed_url.password() {
            config.password(password);
        }
        let mut db_name = parsed_url.path().to_string();
        db_name.replace_range(..1, ""); // strip leading slash
        config.connect_timeout(Duration::from_secs(5));

        let root_connection = Arc::new(PostgreSql::new(config.clone(), 1).unwrap());
        let db_sql = format!("CREATE DATABASE \"{}\";", &db_name);
        let _ = root_connection.query_on_raw_connection("", &db_sql, &[]); // ignoring errors as there's no CREATE DATABASE IF NOT EXISTS in Postgres

        let schema = parsed_url.query_pairs().into_iter().find(|qp| qp.0 == Cow::Borrowed("schema")).expect("schema param is missing").1.to_string();


        config.dbname(&db_name);
        let db_connection = Arc::new(PostgreSql::new(config, connection_limit).unwrap());

        PostgresHelper {
            db_connection,
            schema
        }
    }

    pub fn virtual_variant(
        sql_family: SqlFamily,
        url: &str,
    ) -> Arc<MigrationConnector<DatabaseMigration = SqlMigration>> {
        println!("Loading a virtual connector!");
        // TODO: duplicated from above
        let path = Path::new(&url);
        let schema_name = path
            .file_stem()
            .expect("file url must contain a file name")
            .to_str()
            .unwrap()
            .to_string();
        Arc::new(VirtualSqlMigrationConnector {
            sql_family: sql_family,
            schema_name: schema_name,
        })
    }

    fn create_connector(
        conn: Arc<Connectional>,
        sql_family: SqlFamily,
        schema_name: String,
        file_path: Option<String>,
    ) -> Arc<SqlMigrationConnector> {
        let inspector: Arc<DatabaseInspector> = match sql_family {
            SqlFamily::Sqlite => Arc::new(DatabaseInspector::sqlite_with_connectional(Arc::clone(&conn))),
            SqlFamily::Postgres => Arc::new(DatabaseInspector::postgres_with_connectional(Arc::clone(&conn))),
            _ => unimplemented!(),
        };
        let migration_persistence = Arc::new(SqlMigrationPersistence {
            sql_family,
            connection: Arc::clone(&conn),
            schema_name: schema_name.clone(),
            file_path: file_path.clone(),
        });
        let database_migration_inferrer = Arc::new(SqlDatabaseMigrationInferrer {
            sql_family,
            inspector: Arc::clone(&inspector),
            schema_name: schema_name.to_string(),
        });
        let database_migration_step_applier = Arc::new(SqlDatabaseStepApplier {
            sql_family: sql_family,
            schema_name: schema_name.clone(),
            conn: Arc::clone(&conn),
        });
        let destructive_changes_checker = Arc::new(SqlDestructiveChangesChecker {});
        Arc::new(SqlMigrationConnector {
            file_path,
            sql_family,
            schema_name,
            connectional: Arc::clone(&conn),
            migration_persistence,
            database_migration_inferrer,
            database_migration_step_applier,
            destructive_changes_checker,
            database_inspector: Arc::clone(&inspector),
        })
    }
}

struct PostgresHelper {
    db_connection: Arc<Connectional>,
    schema: String,
}

impl MigrationConnector for SqlMigrationConnector {
    type DatabaseMigration = SqlMigration;

    fn initialize(&self) {
        match self.sql_family {
            SqlFamily::Sqlite => {
                if let Some(file_path) = &self.file_path {
                    let path_buf = PathBuf::from(&file_path);
                    match path_buf.parent() {
                        Some(parent_directory) => {
                            fs::create_dir_all(parent_directory).expect("creating the database folders failed")
                        }
                        None => {}
                    }
                }
            }
            SqlFamily::Postgres => {
                let schema_sql = dbg!(format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", &self.schema_name));                
                self.connectional.query_on_raw_connection(&self.schema_name, &schema_sql, &[]).expect("Creation of Postgres Schema failed");
            }
            SqlFamily::Mysql => unimplemented!(),
        }
        self.migration_persistence.init();
    }

    fn reset(&self) {
        self.migration_persistence.reset();
    }

    fn migration_persistence(&self) -> Arc<MigrationPersistence> {
        Arc::clone(&self.migration_persistence)
    }

    fn database_migration_inferrer(&self) -> Arc<DatabaseMigrationInferrer<SqlMigration>> {
        Arc::clone(&self.database_migration_inferrer)
    }

    fn database_migration_step_applier(&self) -> Arc<DatabaseMigrationStepApplier<SqlMigration>> {
        Arc::clone(&self.database_migration_step_applier)
    }

    fn destructive_changes_checker(&self) -> Arc<DestructiveChangesChecker<SqlMigration>> {
        Arc::clone(&self.destructive_changes_checker)
    }

    fn deserialize_database_migration(&self, json: serde_json::Value) -> SqlMigration {
        serde_json::from_value(json).unwrap()
    }

    fn database_inspector(&self) -> Arc<DatabaseInspector> {
        Arc::clone(&self.database_inspector)
    }
}

struct VirtualSqlMigrationConnector {
    sql_family: SqlFamily,
    schema_name: String,
}
impl MigrationConnector for VirtualSqlMigrationConnector {
    type DatabaseMigration = SqlMigration;

    fn initialize(&self) {}

    fn reset(&self) {}

    fn migration_persistence(&self) -> Arc<MigrationPersistence> {
        Arc::new(EmptyMigrationPersistence {})
    }

    fn database_migration_inferrer(&self) -> Arc<DatabaseMigrationInferrer<SqlMigration>> {
        Arc::new(VirtualSqlDatabaseMigrationInferrer {
            sql_family: self.sql_family,
            schema_name: self.schema_name.clone(),
        })
    }

    fn database_migration_step_applier(&self) -> Arc<DatabaseMigrationStepApplier<SqlMigration>> {
        Arc::new(VirtualSqlDatabaseStepApplier {
            sql_family: self.sql_family,
            schema_name: self.schema_name.clone(),
        })
    }

    fn destructive_changes_checker(&self) -> Arc<DestructiveChangesChecker<SqlMigration>> {
        Arc::new(EmptyDestructiveChangesChecker::new())
    }

    fn deserialize_database_migration(&self, json: serde_json::Value) -> SqlMigration {
        serde_json::from_value(json).unwrap()
    }

    fn database_inspector(&self) -> Arc<DatabaseInspector> {
        Arc::new(DatabaseInspector::empty())
    }
}
