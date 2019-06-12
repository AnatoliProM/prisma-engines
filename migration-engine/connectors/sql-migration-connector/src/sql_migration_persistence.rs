#[allow(unused, dead_code)]
use barrel::types;
use chrono::*;
use migration_connector::*;
use prisma_query::ast::*;
use prisma_query::{Connectional, ResultSet};
use serde_json;
use std::sync::Arc;
use super::SqlFamily;

pub struct SqlMigrationPersistence {
    pub sql_family: SqlFamily,
    pub connection: Arc<Connectional>,
    pub schema_name: String,
    pub file_path: Option<String>,
}

#[allow(unused, dead_code)]
impl MigrationPersistence for SqlMigrationPersistence {
    fn init(&self) {
        println!("SqlMigrationPersistence.init()");
        let mut m = barrel::Migration::new().schema(self.schema_name.clone());

        let barrel_variant = match self.sql_family {
            SqlFamily::Sqlite => {
                m.create_table_if_not_exists(TABLE_NAME, migration_table_setup_sqlite);
                barrel::SqlVariant::Sqlite
            },
            SqlFamily::Postgres => {
                m.create_table(TABLE_NAME, migration_table_setup_postgres);
                barrel::SqlVariant::Pg
            },
            SqlFamily::Mysql => {
                // m.create_table_if_not_exists(TABLE_NAME, |t| migration_table_setup(self.sql_family, t));
                // barrel::SqlVariant::Mysql
                unimplemented!()
            },
        };
        let sql_str = dbg!(m.make_from(barrel_variant));

        let _ = self.connection.query_on_raw_connection(&self.schema_name, &sql_str, &[]);
    }    

    fn reset(&self) {
        println!("SqlMigrationPersistence.reset()");
        let sql_str = format!(r#"DELETE FROM "{}"."_Migration";"#, self.schema_name); // TODO: this is not vendor agnostic yet

        let _ = self.connection.query_on_raw_connection(&self.schema_name, &sql_str, &[]);

        if let Some(ref file_path) = self.file_path {
            let _ = dbg!(std::fs::remove_file(file_path)); // ignore potential errors
        }
    }

    fn last(&self) -> Option<Migration> {
        let conditions = STATUS_COLUMN.equals("Success");
        let query = Select::from_table(self.table())
            .so_that(conditions)
            .order_by(REVISION_COLUMN.descend());

        let result_set = self.connection.query_on_connection(&self.schema_name, query.into()).unwrap();
        parse_rows_new(&result_set).into_iter().next()
    }

    fn load_all(&self) -> Vec<Migration> {
        let query = Select::from_table(self.table());

        let result_set = self.connection.query_on_connection(&self.schema_name, query.into()).unwrap();
        parse_rows_new(&result_set)
    }

    fn by_name(&self, name: &str) -> Option<Migration> {
        let conditions = NAME_COLUMN.equals(name);
        let query = Select::from_table(self.table())
            .so_that(conditions)
            .order_by(REVISION_COLUMN.descend());

        let result_set = self.connection.query_on_connection(&self.schema_name, query.into()).unwrap();
        parse_rows_new(&result_set).into_iter().next()
    }

    fn create(&self, migration: Migration) -> Migration {
        let mut cloned = migration.clone();
        let model_steps_json = serde_json::to_string(&migration.datamodel_steps).unwrap();
        let database_migration_json = serde_json::to_string(&migration.database_migration).unwrap();
        let errors_json = serde_json::to_string(&migration.errors).unwrap();
        let serialized_datamodel = datamodel::render(&migration.datamodel).unwrap();

        let query = Insert::single_into(self.table())
            .value(NAME_COLUMN, migration.name)
            .value(DATAMODEL_COLUMN, serialized_datamodel)
            .value(STATUS_COLUMN, migration.status.code())
            .value(APPLIED_COLUMN, migration.applied)
            .value(ROLLED_BACK_COLUMN, migration.rolled_back)
            .value(DATAMODEL_STEPS_COLUMN, model_steps_json)
            .value(DATABASE_MIGRATION_COLUMN, database_migration_json)
            .value(ERRORS_COLUMN, errors_json)
            .value(
                STARTED_AT_COLUMN,
                self.convert_datetime(migration.started_at),
            )
            .value(FINISHED_AT_COLUMN, ParameterizedValue::Null);

        match self.sql_family {
            SqlFamily::Sqlite => {
                let id = self.connection.execute_on_connection(&self.schema_name, query.into()).unwrap();
                match id {
                    Some(prisma_query::ast::Id::Int(id)) => cloned.revision = id,
                    _ => panic!("This insert must return an int"),
                };
            }
            SqlFamily::Postgres => {
                let result_set = self.connection.query_on_connection(&self.schema_name, query.into()).unwrap();
                result_set.into_iter().next().map(|row| {
                    cloned.revision = row.at_as_integer(0).unwrap() as usize;
                });
            }
            SqlFamily::Mysql => unimplemented!()
        }
        cloned
    }

    fn update(&self, params: &MigrationUpdateParams) {
        let finished_at_value = match params.finished_at {
            Some(x) => self.convert_datetime(x),
            None => ParameterizedValue::Null,
        };
        let errors_json = serde_json::to_string(&params.errors).unwrap();
        let query = Update::table(self.table())
            .set(NAME_COLUMN, params.new_name.clone())
            .set(STATUS_COLUMN, params.status.code())
            .set(APPLIED_COLUMN, params.applied)
            .set(ROLLED_BACK_COLUMN, params.rolled_back)
            .set(ERRORS_COLUMN, errors_json)
            .set(FINISHED_AT_COLUMN, finished_at_value)
            .so_that(
                NAME_COLUMN
                    .equals(params.name.clone())
                    .and(REVISION_COLUMN.equals(params.revision)),
            );

        let _ = self.connection.query_on_connection(&self.schema_name, query.into()).unwrap();
    }
}

fn migration_table_setup_sqlite(t: &mut barrel::Table) {
    migration_table_setup(t, types::date());
}

fn migration_table_setup_postgres(t: &mut barrel::Table) {
    migration_table_setup(t, types::custom("timestamp(3)"));
}

fn migration_table_setup(t: &mut barrel::Table, datetime_type: barrel::types::Type) {
    t.add_column(REVISION_COLUMN, types::primary());
    t.add_column(NAME_COLUMN, types::text());
    t.add_column(DATAMODEL_COLUMN, types::text());
    t.add_column(STATUS_COLUMN, types::text());
    t.add_column(APPLIED_COLUMN, types::integer());
    t.add_column(ROLLED_BACK_COLUMN, types::integer());
    t.add_column(DATAMODEL_STEPS_COLUMN, types::text());
    t.add_column(DATABASE_MIGRATION_COLUMN, types::text());
    t.add_column(ERRORS_COLUMN, types::text());
    t.add_column(STARTED_AT_COLUMN, datetime_type.clone());
    t.add_column(FINISHED_AT_COLUMN, datetime_type.clone().nullable(true));
}


impl SqlMigrationPersistence {
    fn table(&self) -> Table {
        match self.sql_family {
            SqlFamily::Sqlite => {
                // sqlite case. Otherwise prisma-query produces invalid SQL
                TABLE_NAME.to_string().into()
            },
            _ => (self.schema_name.to_string(), TABLE_NAME.to_string()).into(),
        }
    }

    fn convert_datetime(&self, datetime: DateTime<Utc>) -> ParameterizedValue {
        match self.sql_family {
            SqlFamily::Sqlite => ParameterizedValue::Integer(datetime.timestamp_millis()),
            SqlFamily::Postgres => ParameterizedValue::DateTime(datetime),
            _ => unimplemented!(),
        }
    }

}
fn convert_parameterized_date_value(db_value: &ParameterizedValue) -> DateTime<Utc> {
    match db_value {
        ParameterizedValue::Integer(x) => timestamp_to_datetime(*x),
        ParameterizedValue::DateTime(x) => x.clone(),
        x => unimplemented!("Got unsupported value {:?} in date conversion", x),
    }
}

fn timestamp_to_datetime(timestamp: i64) -> DateTime<Utc> {
    let nsecs = ((timestamp % 1000) * 1_000_000) as u32;
    let secs = (timestamp / 1000) as i64;
    let naive = chrono::NaiveDateTime::from_timestamp(secs, nsecs);
    let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);

    datetime
}

fn parse_rows_new(result_set: &ResultSet) -> Vec<Migration> {
    result_set
        .into_iter()
        .map(|row| {
            let datamodel_string: String = row.get_as_string(DATAMODEL_COLUMN).unwrap();
            let datamodel_steps_json: String = row.get_as_string(DATAMODEL_STEPS_COLUMN).unwrap();
            let database_migration_string: String = row.get_as_string(DATABASE_MIGRATION_COLUMN).unwrap();
            let errors_json: String = row.get_as_string(ERRORS_COLUMN).unwrap();
            let finished_at = match row.get(FINISHED_AT_COLUMN) {
                Ok(ParameterizedValue::Null) => None,
                Ok(x) => Some(convert_parameterized_date_value(&x)),
                Err(err) => panic!(format!("{}", err)),
            };

            let datamodel_steps = serde_json::from_str(&datamodel_steps_json).unwrap();
            let datamodel = datamodel::parse(&datamodel_string).unwrap();
            let database_migration_json = serde_json::from_str(&database_migration_string).unwrap();
            let errors: Vec<String> = serde_json::from_str(&errors_json).unwrap();
            println!("{:?}", row.get(STARTED_AT_COLUMN).unwrap());
            Migration {
                name: row.get_as_string(NAME_COLUMN).unwrap(),
                revision: row.get_as_integer(REVISION_COLUMN).unwrap() as usize,
                datamodel: datamodel,
                status: MigrationStatus::from_str(row.get_as_string(STATUS_COLUMN).unwrap()),
                applied: row.get_as_integer(APPLIED_COLUMN).unwrap() as usize,
                rolled_back: row.get_as_integer(ROLLED_BACK_COLUMN).unwrap() as usize,
                datamodel_steps: datamodel_steps,
                database_migration: database_migration_json,
                errors: errors,
                started_at: convert_parameterized_date_value(row.get(STARTED_AT_COLUMN).unwrap()),
                finished_at: finished_at,
            }
        })
        .collect()
}

static TABLE_NAME: &str = "_Migration";
static NAME_COLUMN: &str = "name";
static REVISION_COLUMN: &str = "revision";
static DATAMODEL_COLUMN: &str = "datamodel";
static STATUS_COLUMN: &str = "status";
static APPLIED_COLUMN: &str = "applied";
static ROLLED_BACK_COLUMN: &str = "rolled_back";
static DATAMODEL_STEPS_COLUMN: &str = "datamodel_steps";
static DATABASE_MIGRATION_COLUMN: &str = "database_migration";
static ERRORS_COLUMN: &str = "errors";
static STARTED_AT_COLUMN: &str = "started_at";
static FINISHED_AT_COLUMN: &str = "finished_at";
