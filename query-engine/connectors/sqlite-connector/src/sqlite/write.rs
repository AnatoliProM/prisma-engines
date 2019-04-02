use crate::{mutaction::MutationBuilder, DatabaseWrite, Sqlite};
use connector::*;
use prisma_models::*;
use prisma_query::{
    ast::*,
    visitor::{self, *},
};
use rusqlite::Transaction;
use std::sync::Arc;

impl DatabaseWrite for Sqlite {
    /// Creates a new record to the database.
    ///
    /// A single record with no list arguments. The return value is the `id` of
    /// the created record:
    ///
    /// ```rust
    /// # use prisma_models::*;
    /// # use rusqlite::{Connection, NO_PARAMS};
    /// # use sqlite_connector::*;
    /// # use connector::*;
    /// # use prisma_query::ast::*;
    /// # use serde_json;
    /// # use std::{fs::File, sync::Arc};
    /// # let mut conn = Connection::open_in_memory().unwrap();
    /// #
    /// # let tmp: SchemaTemplate = serde_json::from_reader(File::open("./test_schema.json").unwrap()).unwrap();
    /// # let schema = tmp.build(String::from("test"));
    /// # let trans = conn.transaction().unwrap();
    /// # trans.execute("ATTACH DATABASE './test.db' AS 'test'", NO_PARAMS).unwrap();
    /// # trans.execute("CREATE TABLE IF NOT EXISTS test.User (id Text, name Text);", NO_PARAMS).unwrap();
    /// #
    /// let model = schema.find_model("User").unwrap();
    ///
    /// let mut args = PrismaArgs::default();
    /// args.insert("id", GraphqlId::from("id1"));
    /// args.insert("name", "John");
    ///
    /// assert_eq!(
    ///    GraphqlId::from("id1"),
    ///    Sqlite::create_node(&trans, model, args, vec![]).unwrap(),
    /// );
    ///
    /// assert_eq!(
    ///     1,
    ///     Sqlite::count(&trans, "User", ConditionTree::default()).unwrap()
    /// );
    /// ```
    fn create_node(
        conn: &Transaction,
        model: ModelRef,
        non_list_args: PrismaArgs,
        list_args: Vec<(String, PrismaListValue)>,
    ) -> ConnectorResult<GraphqlId> {
        let (insert, returned_id) = MutationBuilder::create_node(model.clone(), non_list_args);

        Self::execute_one(conn, insert)?;

        let id = match returned_id {
            Some(id) => id,
            None => GraphqlId::Int(conn.last_insert_rowid() as usize),
        };

        for (field_name, list_value) in list_args {
            let field = model.fields().find_from_scalar(&field_name).unwrap();
            let table = field.scalar_list_table();

            if let Some(insert) = MutationBuilder::create_scalar_list_value(table.table(), &list_value, &id) {
                Self::execute_one(conn, insert)?;
            }
        }

        Ok(id)
    }

    fn create_node_and_connect_to_parent(
        conn: &Transaction,
        parent_id: &GraphqlId,
        mutaction: &NestedCreateNode,
    ) -> ConnectorResult<GraphqlId> {
        let related_field = mutaction.relation_field.related_field();

        if related_field.relation_is_inlined_in_parent() {
            let mut prisma_args = mutaction.non_list_args.clone();
            prisma_args.insert(related_field.name.clone(), parent_id.clone());

            Self::create_node(
                conn,
                mutaction.relation_field.related_model().clone(),
                prisma_args,
                mutaction.list_args.clone(),
            )
        } else {
            let id = Self::create_node(
                conn,
                mutaction.relation_field.related_model().clone(),
                mutaction.non_list_args.clone(),
                mutaction.list_args.clone(),
            )?;

            let relation_query = MutationBuilder::create_relation(mutaction.relation_field.clone(), parent_id, &id);

            Self::execute_one(conn, relation_query)?;

            Ok(id)
        }
    }

    fn update_list_args(
        conn: &Transaction,
        ids: Vec<GraphqlId>,
        model: ModelRef,
        list_args: Vec<(String, PrismaListValue)>,
    ) -> ConnectorResult<()> {
        for (field_name, list_value) in list_args {
            let field = model.fields().find_from_scalar(&field_name).unwrap();
            let table = field.scalar_list_table();
            let (deletes, inserts) = MutationBuilder::update_scalar_list_value_by_ids(table, &list_value, ids.to_vec());

            Self::execute_many(conn, deletes)?;
            Self::execute_many(conn, inserts)?;
        }

        Ok(())
    }

    fn update_node<T>(conn: &Transaction, id: GraphqlId, mutaction: &T) -> ConnectorResult<GraphqlId>
    where
        T: SharedUpdateLogic,
    {
        let model = mutaction.model();
        let updating = MutationBuilder::update_by_id(Arc::clone(&model), id.clone(), mutaction.non_list_args())?;

        if let Some(update) = updating {
            Self::execute_one(conn, update)?;
        }

        Self::update_list_args(
            conn,
            vec![id.clone()],
            Arc::clone(&model),
            mutaction.list_args().to_vec(),
        )?;

        Ok(id)
    }

    fn update_nodes(conn: &Transaction, ids: Vec<GraphqlId>, mutaction: &UpdateNodes) -> ConnectorResult<usize> {
        let count = ids.len();

        let updates =
            MutationBuilder::update_by_ids(Arc::clone(&mutaction.model), &mutaction.non_list_args, ids.clone())?;

        Self::execute_many(conn, updates)?;
        Self::update_list_args(conn, ids, Arc::clone(&mutaction.model), mutaction.list_args.clone())?;

        Ok(count)
    }

    /// Execute a single statement in the database.
    fn execute_one<T>(conn: &Transaction, query: T) -> ConnectorResult<()>
    where
        T: Into<Query>,
    {
        let (sql, params) = dbg!(visitor::Sqlite::build(query));
        conn.prepare(&sql)?.execute(&params)?;

        Ok(())
    }

    /// Execute a multiple statements in the database.
    fn execute_many<T>(conn: &Transaction, queries: Vec<T>) -> ConnectorResult<()>
    where
        T: Into<Query>,
    {
        for query in queries {
            Self::execute_one(conn, query)?;
        }

        Ok(())
    }
}
