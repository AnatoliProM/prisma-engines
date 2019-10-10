mod core;
mod directive_list_validator;
mod directive_scope;
mod directive_validator;

pub use self::core::{new_builtin_enum_directives, new_builtin_field_directives, new_builtin_model_directives};

pub use directive_list_validator::DirectiveListValidator;
pub use directive_scope::DirectiveScope;
pub use directive_validator::DirectiveValidator;

/// The error type for directive validators.
pub type Error = crate::errors::ValidationError;
/// The argument type for directive validators.
pub type Args<'a> = crate::common::argument::Arguments<'a>;
