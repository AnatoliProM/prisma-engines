#![deny(warnings)]

#[macro_use]
extern crate log;

mod builders;
mod error;
mod query_ast;
mod query_results;
mod executor;

pub mod ir;

pub use builders::*;
pub use error::*;
pub use query_ast::*;
pub use query_results::*;
pub use executor::*;

pub type CoreResult<T> = Result<T, CoreError>;

/// A type wrapper around read and write queries
#[derive(Debug, Clone)]
pub enum Query {
    Read(ReadQuery),
    Write(WriteQuery),
}