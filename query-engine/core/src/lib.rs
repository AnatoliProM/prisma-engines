#![deny(warnings)]

mod builders;
mod error;
mod query_ast;
mod query_results;
mod read_query_executor;
mod schema;

pub mod ir;

pub use builders::*;
pub use error::*;
pub use query_ast::*;
pub use query_results::*;
pub use read_query_executor::*;
pub use schema::*;

pub type CoreResult<T> = Result<T, CoreError>;
