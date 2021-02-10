use crate::{
    context::PrismaContext,
    request_handlers::{graphql, GraphQlBody, SingleQuery},
    PrismaResponse,
};
use enumflags2::BitFlags;
use migration_core::{api::GenericApi, commands::SchemaPushInput};
use quaint::{
    ast::*,
    connector::ConnectionInfo,
    visitor::{self, Visitor},
};
use sql_migration_connector::SqlMigrationConnector;
use std::sync::Arc;
use test_setup::{
    connectors::Tags, create_mssql_database, create_mysql_database, create_postgres_database, sqlite_test_url,
    TestApiArgs,
};

pub struct QueryEngine {
    context: Arc<PrismaContext>,
}

impl QueryEngine {
    #[allow(dead_code)]
    pub fn new(ctx: PrismaContext) -> Self {
        QueryEngine { context: Arc::new(ctx) }
    }

    pub async fn request(&self, body: impl Into<SingleQuery>) -> serde_json::Value {
        let body = GraphQlBody::Single(body.into());
        let cx = self.context.clone();
        match graphql::handle(body, cx).await {
            PrismaResponse::Single(response) => serde_json::to_value(response).unwrap(),
            _ => unreachable!(),
        }
    }
}

pub struct TestApi {
    migration_api: SqlMigrationConnector,
    config: String,
}

impl TestApi {
    pub async fn new(args: TestApiArgs) -> Self {
        let tags = args.connector_tags;
        let connection_string = (args.url_fn)(args.test_function_name);

        let migration_api = if tags.contains(Tags::Mysql) {
            mysql_migration_connector(&connection_string).await
        } else if tags.contains(Tags::Postgres) {
            postgres_migration_connector(&connection_string).await
        } else if tags.contains(Tags::Sqlite) {
            sqlite_migration_connector(&connection_string).await
        } else if tags.contains(Tags::Mssql) {
            mssql_migration_connector(&connection_string).await
        } else {
            unreachable!()
        };

        TestApi {
            migration_api,
            config: args.datasource_block(&connection_string),
        }
    }

    pub async fn create_engine(&self, datamodel: &str) -> anyhow::Result<QueryEngine> {
        feature_flags::initialize(&[String::from("all")]).unwrap();

        let datamodel_string = format!("{}\n\n{}", self.config, datamodel);
        let dml = datamodel::parse_datamodel(&datamodel_string).unwrap().subject;
        let config = datamodel::parse_configuration(&datamodel_string).unwrap();

        self.migration_api
            .schema_push(&SchemaPushInput {
                schema: datamodel_string,
                force: true,
                assume_empty: true,
            })
            .await?;

        let context = PrismaContext::builder(config.subject, dml)
            .enable_raw_queries(true)
            .build()
            .await
            .unwrap();

        Ok(QueryEngine {
            context: Arc::new(context),
        })
    }

    pub fn connection_info(&self) -> &ConnectionInfo {
        self.migration_api.quaint().connection_info()
    }

    pub fn to_sql_string<'a>(&'a self, query: impl Into<Query<'a>>) -> quaint::Result<(String, Vec<Value>)> {
        match self.connection_info() {
            ConnectionInfo::Postgres(..) => visitor::Postgres::build(query),
            ConnectionInfo::Mysql(..) => visitor::Mysql::build(query),
            ConnectionInfo::Sqlite { .. } | ConnectionInfo::InMemorySqlite { .. } => visitor::Sqlite::build(query),
            ConnectionInfo::Mssql(_) => visitor::Mssql::build(query),
        }
    }
}

pub(super) async fn mysql_migration_connector(url_str: &str) -> SqlMigrationConnector {
    create_mysql_database(&url_str.parse().unwrap()).await.unwrap();
    SqlMigrationConnector::new(url_str, BitFlags::all()).await.unwrap()
}

pub(super) async fn mssql_migration_connector(url_str: &str) -> SqlMigrationConnector {
    create_mssql_database(url_str).await.unwrap();
    SqlMigrationConnector::new(url_str, BitFlags::all()).await.unwrap()
}

pub(super) async fn postgres_migration_connector(url_str: &str) -> SqlMigrationConnector {
    create_postgres_database(&url_str.parse().unwrap()).await.unwrap();
    SqlMigrationConnector::new(url_str, BitFlags::all()).await.unwrap()
}

pub(super) async fn sqlite_migration_connector(db_name: &str) -> SqlMigrationConnector {
    SqlMigrationConnector::new(&sqlite_test_url(db_name), BitFlags::all())
        .await
        .unwrap()
}
