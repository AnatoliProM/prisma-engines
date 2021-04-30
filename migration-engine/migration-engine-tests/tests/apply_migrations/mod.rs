use indoc::formatdoc;
use migration_engine_tests::sql::*;
use pretty_assertions::assert_eq;
use test_macros::test_connector;
use user_facing_errors::{migration_engine::ApplyMigrationError, UserFacingError};

#[test_connector]
async fn apply_migrations_with_an_empty_migrations_folder_works(api: &TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;

    api.apply_migrations(&migrations_directory)
        .send()
        .await?
        .assert_applied_migrations(&[])?;

    Ok(())
}

#[test_connector]
async fn applying_a_single_migration_should_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    let migrations_directory = api.create_migrations_directory()?;

    api.create_migration("init", dm, &migrations_directory).send().await?;

    api.apply_migrations(&migrations_directory)
        .send()
        .await?
        .assert_applied_migrations(&["init"])?;

    api.apply_migrations(&migrations_directory)
        .send()
        .await?
        .assert_applied_migrations(&[])?;

    Ok(())
}

#[test_connector]
async fn applying_two_migrations_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    let migrations_directory = api.create_migrations_directory()?;

    api.create_migration("initial", dm1, &migrations_directory)
        .send()
        .await?;

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    api.create_migration("second-migration", dm2, &migrations_directory)
        .send()
        .await?;

    api.apply_migrations(&migrations_directory)
        .send()
        .await?
        .assert_applied_migrations(&["initial", "second-migration"])?;

    api.apply_migrations(&migrations_directory)
        .send()
        .await?
        .assert_applied_migrations(&[])?;

    Ok(())
}

#[test_connector]
async fn migrations_should_fail_when_the_script_is_invalid(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    let migrations_directory = api.create_migrations_directory()?;

    api.create_migration("initial", dm1, &migrations_directory)
        .send()
        .await?;

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    let second_migration_name = api
        .create_migration("second-migration", dm2, &migrations_directory)
        .send()
        .await?
        .modify_migration(|contents| contents.push_str("\nSELECT (^.^)_n;\n"))?
        .into_output()
        .generated_migration_name
        .unwrap();

    let error = api
        .apply_migrations(&migrations_directory)
        .send()
        .await
        .unwrap_err()
        .to_user_facing()
        .unwrap_known();

    // Assertions about the user facing error.
    {
        let expected_error_message = formatdoc!(
            r#"
                A migration failed to apply. New migrations can not be applied before the error is recovered from. Read more about how to resolve migration issues in a production database: https://pris.ly/d/migrate-resolve

                Migration name: {second_migration_name}

                Database error code: {error_code}

                Database error:
                {message}
                "#,
            second_migration_name = second_migration_name,
            error_code = match api.tags() {
                t if t.contains(Tags::Vitess) => 1105,
                t if t.contains(Tags::Mysql) => 1064,
                t if t.contains(Tags::Mssql) => 102,
                t if t.contains(Tags::Postgres) => 42601,
                t if t.contains(Tags::Sqlite) => 1,
                _ => todo!(),
            },
            message = match api.tags() {
                t if t.contains(Tags::Vitess) => "syntax error at position 10",
                t if t.contains(Tags::Mariadb) => "You have an error in your SQL syntax; check the manual that corresponds to your MariaDB server version for the right syntax to use near \'^.^)_n\' at line 1",
                t if t.contains(Tags::Mysql) => "You have an error in your SQL syntax; check the manual that corresponds to your MySQL server version for the right syntax to use near \'^.^)_n\' at line 1",
                t if t.contains(Tags::Mssql) => "Incorrect syntax near \'^\'.",
                t if t.contains(Tags::Postgres) => "db error: ERROR: syntax error at or near \"^\"",
                t if t.contains(Tags::Sqlite) => "unrecognized token: \"^\"",
                _ => todo!(),
            },
        );

        assert_eq!(error.error_code, ApplyMigrationError::ERROR_CODE);
        assert_eq!(error.message, expected_error_message);
    }

    let mut migrations = api.migration_persistence().list_migrations().await?.unwrap();

    assert_eq!(migrations.len(), 2);

    let second = migrations.pop().unwrap();
    let first = migrations.pop().unwrap();

    first
        .assert_migration_name("initial")?
        .assert_applied_steps_count(1)?
        .assert_success()?;

    second
        .assert_migration_name("second-migration")?
        .assert_applied_steps_count(0)?
        .assert_failed()?;

    Ok(())
}

#[test_connector]
async fn migrations_should_not_reapply_modified_migrations(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    let migrations_directory = api.create_migrations_directory()?;

    let assertions = api
        .create_migration("initial", dm1, &migrations_directory)
        .send()
        .await?;

    api.apply_migrations(&migrations_directory).send().await?;

    assertions.modify_migration(|script| *script = format!("/* this is just a harmless comment */\n{}", script))?;

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    api.create_migration("second-migration", dm2, &migrations_directory)
        .send()
        .await?;

    api.apply_migrations(&migrations_directory)
        .send()
        .await?
        .assert_applied_migrations(&["second-migration"])?;

    Ok(())
}

#[test_connector]
async fn migrations_should_fail_on_an_uninitialized_nonempty_database(api: &TestApi) -> TestResult {
    let dm = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    let directory = api.create_migrations_directory()?;

    api.create_migration("01-init", dm, &directory)
        .send()
        .await?
        .assert_migration_directories_count(1)?;

    let known_error = api
        .apply_migrations(&directory)
        .send()
        .await
        .unwrap_err()
        .to_user_facing()
        .unwrap_known();

    assert_eq!(
        known_error.error_code,
        user_facing_errors::migration_engine::DatabaseSchemaNotEmpty::ERROR_CODE
    );

    Ok(())
}

// Reference for the tables created by PostGIS: https://postgis.net/docs/manual-1.4/ch04.html#id418599
#[test_connector(tags(Postgres))]
async fn migrations_should_succeed_on_an_uninitialized_nonempty_database_with_postgis_tables(
    api: &TestApi,
) -> TestResult {
    let dm = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    let create_spatial_ref_sys_table = "CREATE TABLE IF NOT EXISTS \"spatial_ref_sys\" ( id SERIAL PRIMARY KEY )";
    // The capitalized Geometry is intentional here, because we want the matching to be case-insensitive.
    let create_geometry_columns_table = "CREATE TABLE IF NOT EXiSTS \"Geometry_columns\" ( id SERIAL PRIMARY KEY )";

    api.database().raw_cmd(create_spatial_ref_sys_table).await?;
    api.database().raw_cmd(create_geometry_columns_table).await?;

    let directory = api.create_migrations_directory()?;

    api.create_migration("01-init", dm, &directory)
        .send()
        .await?
        .assert_migration_directories_count(1)?;

    api.apply_migrations(&directory)
        .send()
        .await?
        .assert_applied_migrations(&["01-init"])?;

    Ok(())
}
