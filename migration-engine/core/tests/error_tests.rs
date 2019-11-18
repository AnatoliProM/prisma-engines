mod test_harness;

use migration_connector::steps::{DeleteModel, MigrationStep};
use migration_core::{
    api::{render_error, RpcApi},
    cli,
    commands::{ApplyMigrationCommand, ApplyMigrationInput},
};
use pretty_assertions::assert_eq;
use serde_json::json;
use quaint::prelude::*;
use test_harness::*;
use url::Url;

#[tokio::test]
async fn authentication_failure_must_return_a_known_error_on_postgres() {
    let mut url: Url = postgres_url().parse().unwrap();

    url.set_password(Some("obviously-not-right")).unwrap();

    let dm = format!(
        r#"
            datasource db {{
              provider = "postgres"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let user = url.username();
    let host = url.host().unwrap().to_string();

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "message": format!("Authentication failed against database server at `{host}`, the provided database credentials for `postgres` are not valid.\n\nPlease make sure to provide valid database credentials for the database server at `{host}`.", host = host),
        "meta": {
            "database_user": user,
            "database_host": host,
        },
        "error_code": "P1000"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn authentication_failure_must_return_a_known_error_on_mysql() {
    let mut url: Url = mysql_url().parse().unwrap();

    url.set_password(Some("obviously-not-right")).unwrap();

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let user = url.username();
    let host = url.host().unwrap().to_string();

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "message": format!("Authentication failed against database server at `{host}`, the provided database credentials for `{user}` are not valid.\n\nPlease make sure to provide valid database credentials for the database server at `{host}`.", host = host, user = user),
        "meta": {
            "database_user": user,
            "database_host": host,
        },
        "error_code": "P1000"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn unreachable_database_must_return_a_proper_error_on_mysql() {
    let mut url: Url = mysql_url().parse().unwrap();

    url.set_port(Some(8787)).unwrap();

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let port = url.port().unwrap().to_string();
    let host = url.host().unwrap().to_string();

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "message": format!("Can't reach database server at `{host}`:`{port}`\n\nPlease make sure your database server is running at `{host}`:`{port}`.", host = host, port = port),
        "meta": {
            "database_port": port,
            "database_host": host,
        },
        "error_code": "P1001"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn unreachable_database_must_return_a_proper_error_on_postgres() {
    let mut url: Url = postgres_url().parse().unwrap();

    url.set_port(Some(8787)).unwrap();

    let dm = format!(
        r#"
            datasource db {{
              provider = "postgres"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let host = url.host().unwrap().to_string();
    let port = url.port().unwrap().to_string();

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "message": format!("Can't reach database server at `{host}`:`{port}`\n\nPlease make sure your database server is running at `{host}`:`{port}`.", host = host, port = port),
        "meta": {
            "database_port": port,
            "database_host": host,
        },
        "error_code": "P1001"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn database_does_not_exist_must_return_a_proper_error() {
    let mut url: Url = mysql_url().parse().unwrap();
    let database_name = "notmydatabase";

    url.set_path(database_name);

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let database_location = format!("{}:{}", url.host().unwrap(), url.port().unwrap());

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "message": format!("Database `{database_name}` does not exist on the database server at `{database_location}`.", database_name = database_name, database_location = database_location),
        "meta": {
            "database_name": database_name,
            "database_schema_name": null,
            "database_location": database_location,
        },
        "error_code": "P1003"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn database_already_exists_must_return_a_proper_error() {
    let url = postgres_url();
    let error = get_cli_error(&["migration-engine", "cli", "--datasource", &url, "--create_database"]).await;

    let (host, port) = {
        let url = Url::parse(&url).unwrap();
        (url.host().unwrap().to_string(), url.port().unwrap())
    };

    let json_error = serde_json::to_value(&error).unwrap();

    let expected = json!({
        "message": format!("Database `test-db` already exists on the database server at `{host}:{port}`", host = host, port = port),
        "meta": {
            "database_name": "test-db",
            "database_host": host,
            "database_port": port,
        },
        "error_code": "P1009"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn database_access_denied_must_return_a_proper_error_in_cli() {
    let conn = Quaint::new(&mysql_url()).unwrap();

    conn.execute_raw("DROP USER IF EXISTS jeanmichel", &[]).await.unwrap();
    conn.execute_raw("CREATE USER jeanmichel IDENTIFIED BY '1234'", &[])
        .await
        .unwrap();

    let mut url: Url = mysql_url().parse().unwrap();
    url.set_username("jeanmichel").unwrap();
    url.set_password(Some("1234")).unwrap();
    url.set_path("access_denied_test");

    let error = get_cli_error(&[
        "migration-engine",
        "cli",
        "--datasource",
        url.as_str(),
        "--can_connect_to_database",
    ])
    .await;

    let json_error = serde_json::to_value(&error).unwrap();
    let expected = json!({
        "message": "User `jeanmichel` was denied access on the database `access_denied_test`",
        "meta": {
            "database_user": "jeanmichel",
            "database_name": "access_denied_test",
        },
        "error_code": "P1010",
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn database_access_denied_must_return_a_proper_error_in_rpc() {
    let conn = Quaint::new(&mysql_url()).unwrap();

    conn.execute_raw("DROP USER IF EXISTS jeanmichel", &[]).await.unwrap();
    conn.execute_raw("CREATE USER jeanmichel IDENTIFIED BY '1234'", &[])
        .await
        .unwrap();

    let mut url: Url = mysql_url().parse().unwrap();
    url.set_username("jeanmichel").unwrap();
    url.set_password(Some("1234")).unwrap();
    url.set_path("access_denied_test");

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url,
    );

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();
    let json_error = serde_json::to_value(&render_error(error)).unwrap();

    let expected = json!({
        "message": "User `jeanmichel` was denied access on the database `access_denied_test`",
        "meta": {
            "database_user": "jeanmichel",
            "database_name": "access_denied_test",
        },
        "error_code": "P1010",
    });

    assert_eq!(json_error, expected);
}

#[test_one_connector(connector = "postgres")]
async fn command_errors_must_return_an_unknown_error(api: &TestApi) {
    let input = ApplyMigrationInput {
        migration_id: "the-migration".to_owned(),
        steps: vec![MigrationStep::DeleteModel(DeleteModel {
            model: "abcd".to_owned(),
        })],
        force: Some(true),
    };

    let error = api.execute_command::<ApplyMigrationCommand>(&input).await.unwrap_err();

    let expected_error = user_facing_errors::Error::Unknown(user_facing_errors::UnknownError {
        message: "Failure during a migration command: Generic error. (code: 1, error: The model abcd does not exist in this Datamodel. It is not possible to delete it.)".to_owned(),
        backtrace: None,
    });

    assert_eq!(error, expected_error);
}

async fn get_cli_error(cli_args: &[&str]) -> user_facing_errors::Error {
    let app = cli::clap_app();
    let matches = app.get_matches_from(cli_args);
    let cli_matches = matches.subcommand_matches("cli").expect("cli subcommand is passed");
    let database_url = cli_matches.value_of("datasource").expect("datasource is provided");
    cli::run(&cli_matches, database_url)
        .await
        .map_err(|err| cli::render_error(err))
        .unwrap_err()
}
