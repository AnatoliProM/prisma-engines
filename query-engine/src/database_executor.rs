mod sqlite;

use prisma_common::PrismaResult;
use prisma_models::prelude::*;
use prisma_query::ast::Select;
use rusqlite::Row;

pub use sqlite::Sqlite;

pub trait DatabaseExecutor {
    fn with_rows<F, T>(&self, query: Select, db_name: String, f: F) -> PrismaResult<Vec<T>>
    where
        F: FnMut(&Row) -> T;
}
