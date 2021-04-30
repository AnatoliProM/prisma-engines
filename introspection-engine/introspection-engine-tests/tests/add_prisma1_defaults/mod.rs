use barrel::types;
use indoc::formatdoc;
use introspection_engine_tests::{assert_eq_json, test_api::*, TestResult};
use serde_json::json;
use test_macros::test_connector;

#[test_connector(tags(Postgres, Mysql))]
async fn add_cuid_default(api: &TestApi) -> TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                let typ = if sql_family.is_postgres() {
                    types::varchar(25)
                } else {
                    types::r#char(25)
                };

                t.add_column("id", typ.nullable(false).primary(true));
            });
        })
        .await?;

    let native_type = if sql_family.is_postgres() {
        "@db.VarChar(25)"
    } else {
        "@db.Char(25)"
    };

    let dm = formatdoc! {r#"
        model Book {{
            id  String @id @default(cuid()) {}
        }}
    "#, native_type};

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    let expected = json!([{
        "code": 5,
        "message": "These id fields had a `@default(cuid())` added because we believe the schema was created by Prisma 1.",
        "affected": [
            {
                "model": "Book",
                "field": "id"
            }
        ]
    }]);

    assert_eq_json!(expected, api.introspection_warnings().await?);

    Ok(())
}

#[test_connector(tags(Postgres, Mysql))]
async fn add_uuid_default(api: &TestApi) -> TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                let typ = if sql_family.is_postgres() {
                    types::varchar(36)
                } else {
                    types::r#char(36)
                };

                t.add_column("id", typ.nullable(false).primary(true));
            });
        })
        .await?;

    let native_type = if sql_family.is_postgres() {
        "@db.VarChar(36)"
    } else {
        "@db.Char(36)"
    };

    let dm = formatdoc! {r#"
        model Book {{
            id  String @default(uuid()) @id {}
        }}
    "#, native_type};

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    let expected = json!([{
        "code": 6,
        "message": "These id fields had a `@default(uuid())` added because we believe the schema was created by Prisma 1.",
        "affected": [
            {
                "model": "Book",
                "field": "id"
            }
        ]
    }]);

    assert_eq_json!(expected, api.introspection_warnings().await?);

    Ok(())
}
