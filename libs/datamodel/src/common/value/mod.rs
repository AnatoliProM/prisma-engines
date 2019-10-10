use crate::ast;
use crate::dml;

use crate::common::interpolation::StringInterpolator;
use crate::common::FromStrAndSpan;
use crate::common::{PrismaType, PrismaValue};
use crate::error::DatamodelError;
use crate::FunctionalEvaluator;
use chrono::{DateTime, Utc};
use std::error;

macro_rules! wrap_value (
    ($value:expr, $wrapper:expr, $validator:expr) => ({
        match $value {
            Ok(val) => Ok($wrapper(val)),
            Err(err) => Err(err)
        }
    })
);

#[derive(Debug, Clone)]
pub enum MaybeExpression {
    // The Option is Some if the value came from an env var. The String is then the name of the env var.
    Value(Option<String>, ast::Value),
    Expression(PrismaValue, ast::Span),
}

/// Wraps a value and provides convenience methods for
/// parsing it.
#[derive(Debug)]
pub struct ValueValidator {
    value: MaybeExpression,
}

impl ValueValidator {
    /// Creates a new instance by wrapping a value.
    ///
    /// If the value is a function expression, it is evaluated
    /// recursively.
    pub fn new(value: &ast::Value) -> Result<ValueValidator, DatamodelError> {
        match value {
            ast::Value::StringValue(string, span) => Ok(ValueValidator {
                value: MaybeExpression::Value(None, StringInterpolator::interpolate(string, *span)?),
            }),
            _ => Ok(ValueValidator {
                value: FunctionalEvaluator::new(value).evaluate()?,
            }),
        }
    }

    /// Creates a new type mismatch error for the
    /// value wrapped by this instance.
    fn construct_error(&self, expected_type: &str) -> DatamodelError {
        let description = match &self.value {
            MaybeExpression::Value(_, val) => String::from(ast::describe_value_type(&val)),
            MaybeExpression::Expression(val, _) => val.get_type().to_string(),
        };

        DatamodelError::new_type_mismatch_error(expected_type, &description, &self.raw(), self.span())
    }

    /// Creates a value parser error
    /// from some other parser error.
    fn wrap_error_from_result<T, E: error::Error>(
        &self,
        result: Result<T, E>,
        expected_type: &str,
    ) -> Result<T, DatamodelError> {
        match result {
            Ok(val) => Ok(val),
            Err(err) => Err(DatamodelError::new_value_parser_error(
                expected_type,
                err.description(),
                &self.raw(),
                self.span(),
            )),
        }
    }

    /// Attempts to parse the wrapped value
    /// to a given prisma type.
    pub fn as_type(&self, scalar_type: PrismaType) -> Result<dml::Value, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, _) => match scalar_type {
                PrismaType::Int => wrap_value!(self.as_int(), dml::Value::Int, self),
                PrismaType::Float => wrap_value!(self.as_float(), dml::Value::Float, self),
                PrismaType::Decimal => wrap_value!(self.as_decimal(), dml::Value::Decimal, self),
                PrismaType::Boolean => wrap_value!(self.as_bool(), dml::Value::Boolean, self),
                PrismaType::DateTime => wrap_value!(self.as_date_time(), dml::Value::DateTime, self),
                PrismaType::String => wrap_value!(self.as_str(), dml::Value::String, self),
            },
            MaybeExpression::Expression(expr, _) => {
                if expr.get_type() == scalar_type {
                    Ok(expr.clone())
                } else {
                    Err(self.construct_error(&scalar_type.to_string()))
                }
            }
        }
    }

    /// Parses the wrapped value as a given literal type.
    pub fn parse_literal<T: FromStrAndSpan>(&self) -> Result<T, DatamodelError> {
        T::from_str_and_span(&self.as_constant_literal()?, self.span())
    }

    /// Accesses the raw string representation
    /// of the wrapped value.
    pub fn raw(&self) -> String {
        match &self.value {
            MaybeExpression::Value(_, val) => val.to_string(),
            MaybeExpression::Expression(val, _) => val.to_string(),
        }
    }

    /// Accesses the span of the wrapped value.
    pub fn span(&self) -> ast::Span {
        match &self.value {
            MaybeExpression::Value(_, val) => match val {
                ast::Value::StringValue(_, s) => *s,
                ast::Value::NumericValue(_, s) => *s,
                ast::Value::BooleanValue(_, s) => *s,
                ast::Value::ConstantValue(_, s) => *s,
                ast::Value::Function(_, _, s) => *s,
                ast::Value::Array(_, s) => *s,
                ast::Value::Any(_, s) => *s,
            },
            MaybeExpression::Expression(_, s) => *s,
        }
    }

    /// Tries to convert the wrapped value to a Prisma String.
    pub fn as_str(&self) -> Result<String, DatamodelError> {
        self.as_str_from_env().map(|tuple| tuple.1)
    }

    pub fn as_str_from_env(&self) -> Result<(Option<String>, String), DatamodelError> {
        match &self.value {
            MaybeExpression::Value(env_var, ast::Value::StringValue(value, _)) => {
                Ok((env_var.clone(), value.to_string()))
            }
            MaybeExpression::Value(env_var, ast::Value::Any(value, _)) => Ok((env_var.clone(), value.to_string())),
            _ => Err(self.construct_error("String")),
        }
    }

    /// Tries to convert the wrapped value to a Prisma Integer.
    pub fn as_int(&self) -> Result<i32, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, ast::Value::NumericValue(value, _)) => {
                self.wrap_error_from_result(value.parse::<i32>(), "numeric")
            }
            MaybeExpression::Value(_, ast::Value::Any(value, _)) => {
                self.wrap_error_from_result(value.parse::<i32>(), "numeric")
            }
            _ => Err(self.construct_error("numeric")),
        }
    }

    /// Tries to convert the wrapped value to a Prisma Float.
    pub fn as_float(&self) -> Result<f32, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, ast::Value::NumericValue(value, _)) => {
                self.wrap_error_from_result(value.parse::<f32>(), "numeric")
            }
            MaybeExpression::Value(_, ast::Value::Any(value, _)) => {
                self.wrap_error_from_result(value.parse::<f32>(), "numeric")
            }
            _ => Err(self.construct_error("numeric")),
        }
    }

    // TODO: Ask which decimal type to take.
    /// Tries to convert the wrapped value to a Prisma Decimal.
    pub fn as_decimal(&self) -> Result<f32, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, ast::Value::NumericValue(value, _)) => {
                self.wrap_error_from_result(value.parse::<f32>(), "numeric")
            }
            MaybeExpression::Value(_, ast::Value::Any(value, _)) => {
                self.wrap_error_from_result(value.parse::<f32>(), "numeric")
            }
            _ => Err(self.construct_error("numeric")),
        }
    }

    /// Tries to convert the wrapped value to a Prisma Boolean.
    pub fn as_bool(&self) -> Result<bool, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, ast::Value::BooleanValue(value, _)) => {
                self.wrap_error_from_result(value.parse::<bool>(), "boolean")
            }
            MaybeExpression::Value(_, ast::Value::Any(value, _)) => {
                self.wrap_error_from_result(value.parse::<bool>(), "boolean")
            }
            _ => Err(self.construct_error("boolean")),
        }
    }

    // TODO: Ask which datetime type to use.
    /// Tries to convert the wrapped value to a Prisma DateTime.
    pub fn as_date_time(&self) -> Result<DateTime<Utc>, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, ast::Value::StringValue(value, _)) => {
                self.wrap_error_from_result(value.parse::<DateTime<Utc>>(), "datetime")
            }
            MaybeExpression::Value(_, ast::Value::Any(value, _)) => {
                self.wrap_error_from_result(value.parse::<DateTime<Utc>>(), "datetime")
            }
            _ => Err(self.construct_error("dateTime")),
        }
    }

    /// Unwraps the wrapped value as a constant literal..
    pub fn as_constant_literal(&self) -> Result<String, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, ast::Value::ConstantValue(value, _)) => Ok(value.to_string()),
            MaybeExpression::Value(_, ast::Value::Any(value, _)) => Ok(value.to_string()),
            _ => Err(self.construct_error("constant literal")),
        }
    }

    /// Unwraps the wrapped value as a constant literal..
    pub fn as_array(&self) -> Result<Vec<ValueValidator>, DatamodelError> {
        match &self.value {
            MaybeExpression::Value(_, ast::Value::Array(values, _)) => {
                let mut validators: Vec<ValueValidator> = Vec::new();

                for value in values {
                    validators.push(ValueValidator::new(value)?);
                }

                Ok(validators)
            }
            _ => Ok(vec![ValueValidator {
                value: self.value.clone(),
            }]),
        }
    }
}

pub trait ValueListValidator {
    fn to_str_vec(&self) -> Result<Vec<String>, DatamodelError>;
    fn to_literal_vec(&self) -> Result<Vec<String>, DatamodelError>;
}

impl ValueListValidator for Vec<ValueValidator> {
    fn to_str_vec(&self) -> Result<Vec<String>, DatamodelError> {
        let mut res: Vec<String> = Vec::new();

        for val in self {
            res.push(val.as_str()?);
        }

        Ok(res)
    }

    fn to_literal_vec(&self) -> Result<Vec<String>, DatamodelError> {
        let mut res: Vec<String> = Vec::new();

        for val in self {
            res.push(val.as_constant_literal()?);
        }

        Ok(res)
    }
}

impl Into<ast::Value> for dml::Value {
    fn into(self) -> ast::Value {
        (&self).into()
    }
}

impl Into<ast::Value> for &dml::Value {
    fn into(self) -> ast::Value {
        match self {
            dml::Value::Boolean(true) => ast::Value::BooleanValue(String::from("true"), ast::Span::empty()),
            dml::Value::Boolean(false) => ast::Value::BooleanValue(String::from("false"), ast::Span::empty()),
            dml::Value::String(value) => ast::Value::StringValue(value.clone(), ast::Span::empty()),
            dml::Value::ConstantLiteral(value) => ast::Value::ConstantValue(value.clone(), ast::Span::empty()),
            dml::Value::DateTime(value) => ast::Value::ConstantValue(value.to_rfc3339(), ast::Span::empty()),
            dml::Value::Decimal(value) => ast::Value::NumericValue(value.to_string(), ast::Span::empty()),
            dml::Value::Float(value) => ast::Value::NumericValue(value.to_string(), ast::Span::empty()),
            dml::Value::Int(value) => ast::Value::NumericValue(value.to_string(), ast::Span::empty()),
            dml::Value::Expression(name, _, args) => ast::Value::Function(
                name.clone(),
                args.iter().map(|a| a.into()).collect(),
                ast::Span::empty(),
            ),
        }
    }
}
