#![allow(non_snake_case)]
mod test_harness;
use barrel::{backend::Sqlite as Squirrel, types, Migration};
use database_inspector::*;
use rusqlite::{Connection, Result, NO_PARAMS};
use std::{thread, time};
use test_harness::*;

const SCHEMA: &str = "migration_engine";

#[test]
fn adding_a_model_for_an_existing_table_must_work() {
    run_test_with_engine(|engine| {
        let initial_result = setup(|mut migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        });
        let dm = r#"
            model Blog {
                id: Int @primary
            }
        "#;
        let result = migrate_to(&engine, &dm);
        assert_eq!(initial_result, result);
    });
}

#[test]
fn bigint_columns_must_work() {
    // TODO: port when barrel supports arbitray primary keys
}

#[test]
fn removing_a_model_for_a_table_that_is_already_deleted_must_work() {
    run_test_with_engine(|engine| {
        let dm1 = r#"
            model Blog {
                id: Int @primary
            }

            model Post {
                id: Int @primary
            }
        "#;
        let initial_result = migrate_to(&engine, &dm1);
        assert_eq!(initial_result.table("Post").is_some(), true);

        let result = execute(|mut migration| {
            migration.drop_table("Post");
        });
        assert_eq!(result.table("Post").is_some(), false);

        let dm2 = r#"
            model Blog {
                id: Int @primary
            }
        "#;
        let final_result = migrate_to(&engine, &dm2);
        assert_eq!(result, final_result);
    });
}

#[test]
fn creating_a_field_for_an_existing_column_with_a_compatible_type_must_work() {
    run_test_with_engine(|engine| {
        let initial_result = setup(|mut migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("title", types::text());
            });
        });
        let dm = r#"
            model Blog {
                id: Int @primary
                title: String
            }
        "#;
        let result = migrate_to(&engine, &dm);
        assert_eq!(initial_result, result);
    });
}

#[test]
fn creating_a_field_for_an_existing_column_and_changing_its_type_must_work() {
    run_test_with_engine(|engine| {
        let initial_result = setup(|mut migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("title", types::integer().nullable(true));
            });
        });
        let initial_column = initial_result.table_bang("Blog").column_bang("title");
        assert_eq!(initial_column.tpe, ColumnType::Int);
        assert_eq!(initial_column.is_required, false);

        let dm = r#"
            model Blog {
                id: Int @primary
                title: String @unique
            }
        "#;
        let result = migrate_to(&engine, &dm);
        let column = result.table_bang("Blog").column_bang("title");
        assert_eq!(column.tpe, ColumnType::String);
        assert_eq!(column.is_required, true);
        // TODO: assert uniqueness
    });
}

#[test]
fn creating_a_field_for_an_existing_column_and_simultaneously_making_it_optional() {
    run_test_with_engine(|engine| {
        let initial_result = setup(|mut migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("title", types::text());
            });
        });
        let initial_column = initial_result.table_bang("Blog").column_bang("title");
        assert_eq!(initial_column.is_required, true);

        let dm = r#"
            model Blog {
                id: Int @primary
                title: String?
            }
        "#;
        let result = migrate_to(&engine, &dm);
        let column = result.table_bang("Blog").column_bang("title");
        assert_eq!(column.is_required, false);
    });
}

#[test]
fn creating_a_scalar_list_field_for_an_existing_table_must_work() {
    run_test_with_engine(|engine| {
        let dm1 = r#"
            model Blog {
                id: Int @primary
            }
        "#;
        let initial_result = migrate_to(&engine, &dm1);
        assert_eq!(initial_result.table("Blog_tags").is_some(), false);

        let result = execute(|mut migration| {
            migration.create_table("Blog_tags", |t| {
                t.add_column("nodeId", types::foreign("Blog(id)"));
                t.add_column("position", types::integer());
                t.add_column("value", types::text());
            });
        });
        assert_eq!(result.table("Blog_tags").is_some(), true);

        let dm2 = r#"
            model Blog {
                id: Int @primary
                tags: String[]
            }
        "#;
        let final_result = migrate_to(&engine, &dm2);
        assert_eq!(result, final_result);
    });
}

#[test]
fn delete_a_field_for_a_non_existent_column_must_work() {
    run_test_with_engine(|engine| {
        let dm1 = r#"
            model Blog {
                id: Int @primary
                title: String
            }
        "#;
        let initial_result = migrate_to(&engine, &dm1);
        assert_eq!(initial_result.table_bang("Blog").column("title").is_some(), true);

        let result = execute(|mut migration| {
            // sqlite does not support dropping columns. So we are emulating it..
            migration.drop_table("Blog");
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        });
        assert_eq!(result.table_bang("Blog").column("title").is_some(), false);

        let dm2 = r#"
            model Blog {
                id: Int @primary
            }
        "#;
        let final_result = migrate_to(&engine, &dm2);
        assert_eq!(result, final_result);
    });
}

#[test]
fn deleting_a_scalar_list_field_for_a_non_existent_list_table_must_work() {
    run_test_with_engine(|engine| {
        let dm1 = r#"
            model Blog {
                id: Int @primary
                tags: String[]
            }
        "#;
        let initial_result = migrate_to(&engine, &dm1);
        assert_eq!(initial_result.table("Blog_tags").is_some(), true);

        let result = execute(|mut migration| {
            migration.drop_table("Blog_tags");
        });
        assert_eq!(result.table("Blog_tags").is_some(), false);

        let dm2 = r#"
            model Blog {
                id: Int @primary
            }
        "#;
        let final_result = migrate_to(&engine, &dm2);
        assert_eq!(result, final_result);
    });
}

#[test]
fn updating_a_field_for_a_non_existent_column() {
    run_test_with_engine(|engine| {
        let dm1 = r#"
            model Blog {
                id: Int @primary
                title: String
            }
        "#;
        let initial_result = migrate_to(&engine, &dm1);
        let initial_column = initial_result.table_bang("Blog").column_bang("title");
        assert_eq!(initial_column.tpe, ColumnType::String);

        let result = execute(|mut migration| {
            // sqlite does not support dropping columns. So we are emulating it..
            migration.drop_table("Blog");
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        });
        assert_eq!(result.table_bang("Blog").column("title").is_some(), false);

        let dm2 = r#"
            model Blog {
                id: Int @primary
                title: Int @unique
            }
        "#;
        let final_result = migrate_to(&engine, &dm2);
        let final_column = final_result.table_bang("Blog").column_bang("title");
        assert_eq!(final_column.tpe, ColumnType::Int);
        // TODO: assert uniqueness
    });
}

#[test]
fn renaming_a_field_where_the_column_was_already_renamed_must_work() {
    run_test_with_engine(|engine| {
        let dm1 = r#"
            model Blog {
                id: Int @primary
                title: String
            }
        "#;
        let initial_result = migrate_to(&engine, &dm1);
        let initial_column = initial_result.table_bang("Blog").column_bang("title");
        assert_eq!(initial_column.tpe, ColumnType::String);

        let result = execute(|mut migration| {
            // sqlite does not support renaming columns. So we are emulating it..
            migration.drop_table("Blog");
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("new_title", types::text());
            });
        });
        assert_eq!(result.table_bang("Blog").column("new_title").is_some(), true);

        let dm2 = r#"
            model Blog {
                id: Int @primary
                title: Float @db(name: "new_title")
            }
        "#;
        let final_result = migrate_to(&engine, &dm2);
        let final_column = final_result.table_bang("Blog").column_bang("new_title");
        assert_eq!(final_column.tpe, ColumnType::Float);
        assert_eq!(final_result.table_bang("Blog").column("title").is_some(), false);
        // TODO: assert uniqueness
    });
}

// FIXME: this was copy pasted from tests.rs from database-inspector
fn setup<F>(mut migrationFn: F) -> DatabaseSchema
where
    F: FnMut(&mut Migration) -> (),
{
    execute_internal(migrationFn, true)
}

fn execute<F>(mut migrationFn: F) -> DatabaseSchema
where
    F: FnMut(&mut Migration) -> (),
{
    execute_internal(migrationFn, false)
}

fn execute_internal<F>(mut migrationFn: F, delete_db_file: bool) -> DatabaseSchema
where
    F: FnMut(&mut Migration) -> (),
{
    let connection = Connection::open_in_memory()
        .and_then(|c| {
            let server_root = std::env::var("SERVER_ROOT").expect("Env var SERVER_ROOT required but not found.");
            let path = format!("{}/db", server_root);
            let database_file_path = dbg!(format!("{}/{}.db", path, SCHEMA));
            if delete_db_file {
                std::fs::remove_file(database_file_path.clone()); // ignore potential errors
                thread::sleep(time::Duration::from_millis(100));
            }

            c.execute("ATTACH DATABASE ? AS ?", &[database_file_path.as_ref(), SCHEMA])
                .map(|_| c)
        })
        .and_then(|c| {
            let mut migration = Migration::new().schema(SCHEMA);
            migrationFn(&mut migration);
            let full_sql = migration.make::<Squirrel>();
            for sql in full_sql.split(";") {
                dbg!(sql);
                if (sql != "") {
                    c.execute(&sql, NO_PARAMS).unwrap();
                }
            }
            Ok(c)
        })
        .unwrap();

    let inspector = DatabaseInspectorImpl::new(connection);
    let mut result = inspector.introspect(&SCHEMA.to_string());
    // the presence of the _Migration table makes assertions harder. Therefore remove it.
    result.tables = result.tables.into_iter().filter(|t| t.name != "_Migration").collect();
    result
}
