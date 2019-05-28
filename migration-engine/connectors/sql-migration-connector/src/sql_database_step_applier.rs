use crate::*;
use barrel::Migration as BarrelMigration;
use migration_connector::*;
use rusqlite::{Connection, NO_PARAMS};

pub struct SqlDatabaseStepApplier {
    connection: Connection,
    schema_name: String,
}

impl SqlDatabaseStepApplier {
    pub fn new(connection: Connection, schema_name: String) -> Self {
        SqlDatabaseStepApplier {
            connection,
            schema_name,
        }
    }
}

#[allow(unused, dead_code)]
impl DatabaseMigrationStepApplier<SqlMigrationStep> for SqlDatabaseStepApplier {
    fn apply(&self, step: SqlMigrationStep) {
        let mut migration = BarrelMigration::new().schema(self.schema_name.clone());

        let sql_string = match dbg!(step) {
            SqlMigrationStep::CreateTable(CreateTable {
                name,
                columns,
                primary_columns,
            }) => {
                migration.create_table(name, move |t| {
                    for column in columns.clone() {
                        let tpe = column_description_to_barrel_type(&column);
                        t.add_column(column.name, tpe);
                    }
                    if primary_columns.len() > 0 {
                        let column_names: Vec<String> = primary_columns
                            .clone()
                            .into_iter()
                            .map(|col| format!("\"{}\"", col))
                            .collect();
                        t.inject_custom(format!("PRIMARY KEY ({})", column_names.join(",")));
                    }
                });
                self.make_sql_string(migration)
            }
            SqlMigrationStep::DropTable(DropTable { name }) => {
                migration.drop_table(name);
                self.make_sql_string(migration)
            }
            SqlMigrationStep::RenameTable { name, new_name } => {
                // TODO: use barrel again when the rename bug is fixed;
                // migration.rename_table(name.to_string(), new_name.to_string());
                // self.make_sql_string(migration)
                format!(
                    r#"ALTER TABLE "{}"."{}" RENAME TO "{}";"#,
                    &self.schema_name, name, new_name
                )
            }
            SqlMigrationStep::AlterTable(AlterTable { table, changes }) => {
                migration.change_table(table, move |t| {
                    for change in changes.clone() {
                        match change {
                            TableChange::AddColumn(AddColumn { column }) => {
                                let tpe = column_description_to_barrel_type(&column);
                                t.add_column(column.name, tpe);
                            }
                            TableChange::DropColumn(DropColumn { name }) => t.drop_column(name),
                            TableChange::AlterColumn(AlterColumn { name, column }) => {
                                t.drop_column(name);
                                let tpe = column_description_to_barrel_type(&column);
                                t.add_column(column.name, tpe);
                            }
                        }
                    }
                });
                self.make_sql_string(migration)
            }
            SqlMigrationStep::RawSql(sql) => sql,
            x => {
                unimplemented!("{:?} not implemented yet here", x);
            }
        };
        dbg!(&sql_string);
        let result = self.connection.execute(&sql_string, NO_PARAMS);
        // TODO: this does not evaluate the results of the PRAGMA foreign_key_check
        match dbg!(result) {
            Ok(_) => {}
            Err(rusqlite::Error::ExecuteReturnedResults) => {} // renames return results and crash the driver ..
            e @ Err(_) => {
                e.unwrap();
                {}
            }
        }
    }
}

impl SqlDatabaseStepApplier {
    fn make_sql_string(&self, migration: BarrelMigration) -> String {
        // TODO: this should pattern match on the connector type once we have this information available
        migration.make::<barrel::backend::Sqlite>()
    }
}

fn column_description_to_barrel_type(column_description: &ColumnDescription) -> barrel::types::Type {
    // TODO: add foreign keys for non integer types once this is available in barrel
    let tpe = match &column_description.foreign_key {
        Some(fk) => {
            //barrel::types::foreign(string_to_static_str(format!("{}({})", fk.table, fk.column))),
            let tpe_str = print_type(column_description.tpe);
            let complete = dbg!(format!("{} REFERENCES {}({})", tpe_str, fk.table, fk.column));
            barrel::types::custom(string_to_static_str(complete))
        }
        None => match column_description.tpe {
            ColumnType::Boolean => barrel::types::boolean(),
            ColumnType::DateTime => barrel::types::date(),
            ColumnType::Float => barrel::types::float(),
            ColumnType::Int => barrel::types::integer(),
            ColumnType::String => barrel::types::text(),
        },
    };
    tpe.nullable(!column_description.required)
}
// TODO: this is copied from barrel
fn print_type(t: ColumnType) -> String {
    match t {
        ColumnType::Boolean => format!("BOOLEAN"),
        ColumnType::DateTime => format!("DATE"),
        ColumnType::Float => format!("REAL"),
        ColumnType::Int => format!("INTEGER"),
        ColumnType::String => format!("TEXT"),
    }
    // match t {
    //     Text => format!("TEXT"),
    //     Varchar(l) => match l {
    //         0 => format!("VARCHAR"), // For "0" remove the limit
    //         _ => format!("VARCHAR({})", l),
    //     },
    //     Primary => format!("INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT"),
    //     Integer => format!("INTEGER"),
    //     Float => format!("REAL"),
    //     Double => format!("DOUBLE"),
    //     UUID => unimplemented!(),
    //     Boolean => format!("BOOLEAN"),
    //     Date => format!("DATE"),
    //     Json => panic!("Json is not supported by Sqlite3"),
    //     Binary => format!("BINARY"),
    //     Foreign(t) => format!("INTEGER REFERENCES {}", t),
    //     Custom(t) => format!("{}", t),
    //     Array(meh) => format!("{}[]", Sqlite::print_type(*meh)),
    //     Index(_) => unimplemented!(),
    // }
}

fn string_to_static_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}
