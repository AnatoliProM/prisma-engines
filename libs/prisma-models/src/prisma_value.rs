use crate::{DomainError, DomainResult, EnumValue};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{convert::TryFrom, fmt, string::FromUtf8Error};
use uuid::Uuid;

pub type PrismaListValue = Vec<PrismaValue>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub enum GraphqlId {
    String(String),
    Int(usize),
    UUID(Uuid),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "gcValueType", content = "value")]
pub enum PrismaValue {
    #[serde(rename = "string")]
    String(String),

    #[serde(rename = "float")]
    Float(f64),

    #[serde(rename = "bool")]
    Boolean(bool),

    #[serde(rename = "datetime")]
    DateTime(DateTime<Utc>),

    #[serde(rename = "enum")]
    Enum(EnumValue),

    #[serde(rename = "json")]
    Json(Value),

    #[serde(rename = "int")]
    Int(i64),

    #[serde(rename = "null")]
    Null,

    #[serde(rename = "uuid")]
    Uuid(Uuid),

    #[serde(rename = "graphQlId")]
    GraphqlId(GraphqlId),

    #[serde(rename = "list")]
    List(PrismaListValue),
}

impl PrismaValue {
    pub fn is_null(&self) -> bool {
        match self {
            PrismaValue::Null => true,
            _ => false,
        }
    }
}

impl fmt::Display for PrismaValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PrismaValue::String(x) => x.fmt(f),
            PrismaValue::Float(x) => x.fmt(f),
            PrismaValue::Boolean(x) => x.fmt(f),
            PrismaValue::DateTime(x) => x.fmt(f),
            PrismaValue::Enum(x) => x.as_string().fmt(f),
            PrismaValue::Json(x) => x.fmt(f),
            PrismaValue::Int(x) => x.fmt(f),
            PrismaValue::Null => "null".fmt(f),
            PrismaValue::Uuid(x) => x.fmt(f),
            PrismaValue::GraphqlId(x) => match x {
                GraphqlId::String(x) => x.fmt(f),
                GraphqlId::Int(x) => x.fmt(f),
                GraphqlId::UUID(x) => x.fmt(f),
            },
            PrismaValue::List(x) => {
                let as_string = format!("{:?}", x);
                as_string.fmt(f)
            }
        }
    }
}

impl From<&str> for PrismaValue {
    fn from(s: &str) -> Self {
        PrismaValue::from(s.to_string())
    }
}

impl From<String> for PrismaValue {
    fn from(s: String) -> Self {
        PrismaValue::String(s)
    }
}

impl From<f64> for PrismaValue {
    fn from(f: f64) -> Self {
        PrismaValue::Float(f)
    }
}

impl From<f32> for PrismaValue {
    fn from(f: f32) -> Self {
        PrismaValue::Float(f64::from(f))
    }
}

impl From<bool> for PrismaValue {
    fn from(b: bool) -> Self {
        PrismaValue::Boolean(b)
    }
}

impl From<i32> for PrismaValue {
    fn from(i: i32) -> Self {
        PrismaValue::Int(i64::from(i))
    }
}

impl From<i64> for PrismaValue {
    fn from(i: i64) -> Self {
        PrismaValue::Int(i)
    }
}

impl From<usize> for PrismaValue {
    fn from(u: usize) -> Self {
        PrismaValue::Int(u as i64)
    }
}

impl From<Uuid> for PrismaValue {
    fn from(s: Uuid) -> Self {
        PrismaValue::Uuid(s)
    }
}

impl From<PrismaListValue> for PrismaValue {
    fn from(s: PrismaListValue) -> Self {
        PrismaValue::List(s)
    }
}

impl From<GraphqlId> for PrismaValue {
    fn from(id: GraphqlId) -> PrismaValue {
        PrismaValue::GraphqlId(id)
    }
}

impl From<&GraphqlId> for PrismaValue {
    fn from(id: &GraphqlId) -> PrismaValue {
        PrismaValue::GraphqlId(id.clone())
    }
}

impl TryFrom<PrismaValue> for PrismaListValue {
    type Error = DomainError;

    fn try_from(s: PrismaValue) -> DomainResult<PrismaListValue> {
        match s {
            PrismaValue::List(l) => Ok(l),
            PrismaValue::Null => Ok(Vec::new()),
            _ => Err(DomainError::ConversionFailure("PrismaValue", "PrismaListValue")),
        }
    }
}

impl TryFrom<PrismaValue> for GraphqlId {
    type Error = DomainError;

    fn try_from(value: PrismaValue) -> DomainResult<GraphqlId> {
        match value {
            PrismaValue::GraphqlId(id) => Ok(id),
            PrismaValue::Int(i) => Ok(GraphqlId::from(i)),
            PrismaValue::String(s) => Ok(GraphqlId::from(s)),
            PrismaValue::Uuid(u) => Ok(GraphqlId::from(u)),
            _ => Err(DomainError::ConversionFailure("PrismaValue", "GraphqlId")),
        }
    }
}

impl TryFrom<&PrismaValue> for GraphqlId {
    type Error = DomainError;

    fn try_from(value: &PrismaValue) -> DomainResult<GraphqlId> {
        match value {
            PrismaValue::GraphqlId(id) => Ok(id.clone()),
            PrismaValue::Int(i) => Ok(GraphqlId::from(*i)),
            PrismaValue::String(s) => Ok(GraphqlId::from(s.clone())),
            PrismaValue::Uuid(u) => Ok(GraphqlId::from(*u)),
            _ => Err(DomainError::ConversionFailure("PrismaValue", "GraphqlId")),
        }
    }
}

impl TryFrom<PrismaValue> for i64 {
    type Error = DomainError;

    fn try_from(value: PrismaValue) -> DomainResult<i64> {
        match value {
            PrismaValue::Int(i) => Ok(i),
            _ => Err(DomainError::ConversionFailure("PrismaValue", "i64")),
        }
    }
}

impl From<&str> for GraphqlId {
    fn from(s: &str) -> Self {
        GraphqlId::from(s.to_string())
    }
}

impl From<String> for GraphqlId {
    fn from(s: String) -> Self {
        GraphqlId::String(s)
    }
}

impl TryFrom<Vec<u8>> for GraphqlId {
    type Error = FromUtf8Error;

    fn try_from(v: Vec<u8>) -> Result<GraphqlId, Self::Error> {
        Ok(GraphqlId::String(String::from_utf8(v)?))
    }
}

impl From<usize> for GraphqlId {
    fn from(id: usize) -> Self {
        GraphqlId::Int(id)
    }
}

impl From<i64> for GraphqlId {
    fn from(id: i64) -> Self {
        GraphqlId::Int(id as usize)
    }
}

impl From<u64> for GraphqlId {
    fn from(id: u64) -> Self {
        GraphqlId::Int(id as usize)
    }
}

impl From<Uuid> for GraphqlId {
    fn from(uuid: Uuid) -> Self {
        GraphqlId::UUID(uuid)
    }
}
