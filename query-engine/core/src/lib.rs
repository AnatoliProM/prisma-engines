#![warn(warnings)] // todo deny warnings once done

// #[macro_use]
// extern crate log;

#[macro_use]
extern crate debug_stub_derive;

#[macro_use]
extern crate lazy_static;

mod error;

pub mod executor;
pub mod query_builders;
pub mod query_document;
pub mod response_ir;
pub mod schema;

pub use error::*;
pub use executor::QueryExecutor;
pub use schema::*;

use connector::{Query, ReadQueryResult, WriteQueryResult};

pub type CoreResult<T> = Result<T, CoreError>;

/// Temporary type to work around current dependent execution limitations.
pub type QueryPair = (Query, ResultResolutionStrategy);

#[derive(Debug)]
pub enum ResultResolutionStrategy {
    /// Resolve the actual result by evaluating another query.
    Dependent(Box<QueryPair>),

    /// Serialize the result as-is into the specified output type.
    Serialize(OutputTypeRef),
}

#[derive(Debug)]
pub enum ResultPair {
    Read(ReadQueryResult, OutputTypeRef),
    Write(WriteQueryResult, OutputTypeRef)
}