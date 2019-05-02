mod create;
mod delete;
mod delete_many;
mod nested;
mod relation;
mod update;
mod update_many;

use crate::{database::SqlDatabase, Transaction, Transactional};
use connector::{error::ConnectorError, mutaction::*, ConnectorResult, DatabaseMutactionExecutor};
use serde_json::Value;
use std::sync::Arc;

impl<T> DatabaseMutactionExecutor for SqlDatabase<T>
where
    T: Transactional,
{
    fn execute(
        &self,
        db_name: String,
        mutaction: TopLevelDatabaseMutaction,
    ) -> ConnectorResult<DatabaseMutactionResult> {
        self.executor.with_transaction(&db_name, |conn: &mut Transaction| {
            fn create(conn: &mut Transaction, cn: &CreateNode) -> ConnectorResult<DatabaseMutactionResult> {
                let parent_id = create::execute(conn, Arc::clone(&cn.model), &cn.non_list_args, &cn.list_args)?;
                nested::execute(conn, &cn.nested_mutactions, &parent_id)?;

                Ok(DatabaseMutactionResult {
                    identifier: Identifier::Id(parent_id),
                    typ: DatabaseMutactionResultType::Create,
                })
            }

            fn update(conn: &mut Transaction, un: &UpdateNode) -> ConnectorResult<DatabaseMutactionResult> {
                let parent_id = update::execute(conn, &un.where_, &un.non_list_args, &un.list_args)?;
                nested::execute(conn, &un.nested_mutactions, &parent_id)?;

                Ok(DatabaseMutactionResult {
                    identifier: Identifier::Id(parent_id),
                    typ: DatabaseMutactionResultType::Update,
                })
            }

            match mutaction {
                TopLevelDatabaseMutaction::CreateNode(ref cn) => create(conn, cn),
                TopLevelDatabaseMutaction::UpdateNode(ref un) => update(conn, un),
                TopLevelDatabaseMutaction::UpsertNode(ref ups) => match conn.find_id(&ups.where_) {
                    Err(_e @ ConnectorError::NodeNotFoundForWhere { .. }) => create(conn, &ups.create),
                    Err(e) => return Err(e),
                    Ok(_) => update(conn, &ups.update),
                },
                TopLevelDatabaseMutaction::UpdateNodes(ref uns) => {
                    let count = update_many::execute(
                        conn,
                        Arc::clone(&uns.model),
                        &uns.filter,
                        &uns.non_list_args,
                        &uns.list_args,
                    )?;

                    Ok(DatabaseMutactionResult {
                        identifier: Identifier::Count(count),
                        typ: DatabaseMutactionResultType::Many,
                    })
                }
                TopLevelDatabaseMutaction::DeleteNode(ref dn) => {
                    let node = delete::execute(conn, &dn.where_)?;

                    Ok(DatabaseMutactionResult {
                        identifier: Identifier::Node(node),
                        typ: DatabaseMutactionResultType::Delete,
                    })
                }
                TopLevelDatabaseMutaction::DeleteNodes(ref dns) => {
                    let count = delete_many::execute(conn, Arc::clone(&dns.model), &dns.filter)?;

                    Ok(DatabaseMutactionResult {
                        identifier: Identifier::Count(count),
                        typ: DatabaseMutactionResultType::Many,
                    })
                }
                TopLevelDatabaseMutaction::ResetData(ref rd) => {
                    conn.truncate(Arc::clone(&rd.project))?;

                    Ok(DatabaseMutactionResult {
                        identifier: Identifier::None,
                        typ: DatabaseMutactionResultType::Unit,
                    })
                }
            }
        })
    }

    fn execute_raw(&self, _query: String) -> ConnectorResult<Value> {
        Ok(Value::String("hello world!".to_string()))
    }
}
