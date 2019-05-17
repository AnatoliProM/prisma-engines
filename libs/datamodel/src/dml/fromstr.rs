use crate::ast::Span;
use crate::errors::ValidationError;

pub trait FromStrAndSpan: Sized {
    fn from_str_and_span(s: &str, span: &Span) -> Result<Self, ValidationError>;
}
