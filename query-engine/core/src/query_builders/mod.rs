//! Query builders module

mod error;
mod filters;
mod query_builder;
mod read;
mod utils;
mod write;

pub use error::*;
pub use filters::*;
pub use query_builder::*;
pub use read::*;
pub use utils::*;
pub use write::*;

/// Query builder sub-result type.
pub type QueryBuilderResult<T> = Result<T, QueryValidationError>;

pub trait Builder<T> {
    fn build(self) -> QueryBuilderResult<T>;
}
