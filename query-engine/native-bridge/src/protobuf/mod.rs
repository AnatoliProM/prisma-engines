mod envelope;
mod filter;
mod input;
mod interface;
mod order_by;
mod query_arguments;

pub mod prelude;
pub use envelope::ProtoBufEnvelope;
pub use filter::*;
pub use input::*;
pub use interface::ProtoBufInterface;

use crate::Error as CrateError;
use chrono::prelude::*;
use prelude::*;
use prisma_common::{error::Error, PrismaResult};
use prisma_models::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

pub mod prisma {
    include!(concat!(env!("OUT_DIR"), "/prisma.rs"));
}

impl From<prisma::NodesResult> for prisma::Result {
    fn from(res: prisma::NodesResult) -> prisma::Result {
        prisma::Result {
            value: Some(result::Value::NodesResult(res)),
        }
    }
}

impl From<usize> for prisma::Result {
    fn from(res: usize) -> prisma::Result {
        prisma::Result {
            value: Some(result::Value::Integer(res as u32)),
        }
    }
}

impl From<prisma::ScalarListValuesResult> for prisma::Result {
    fn from(res: prisma::ScalarListValuesResult) -> prisma::Result {
        prisma::Result {
            value: Some(result::Value::ScalarListResults(res)),
        }
    }
}

impl RpcResponse {
    pub fn header() -> Header {
        Header {
            type_name: String::from("RpcResponse"),
        }
    }

    pub fn empty() -> RpcResponse {
        RpcResponse {
            header: Self::header(),
            response: Some(rpc::Response::Result(prisma::Result { value: None })),
        }
    }

    pub fn ok<T>(result: T) -> RpcResponse
    where
        T: Into<prisma::Result>,
    {
        RpcResponse {
            header: Self::header(),
            response: Some(rpc::Response::Result(result.into())),
        }
    }

    pub fn ok_raw(result: ExecuteRawResult) -> RpcResponse {
        RpcResponse {
            header: Self::header(),
            response: Some(rpc::Response::Result(prisma::Result {
                value: Some(result::Value::ExecuteRawResult(result)),
            })),
        }
    }

    pub fn error(error: CrateError) -> RpcResponse {
        RpcResponse {
            header: Self::header(),
            response: Some(rpc::Response::Error(ProtoError {
                value: Some(error.into()),
            })),
        }
    }
}

impl From<prisma::order_by::SortOrder> for SortOrder {
    fn from(so: prisma::order_by::SortOrder) -> SortOrder {
        match so {
            prisma::order_by::SortOrder::Asc => SortOrder::Ascending,
            prisma::order_by::SortOrder::Desc => SortOrder::Descending,
        }
    }
}

impl From<ValueContainer> for PrismaValue {
    fn from(container: ValueContainer) -> PrismaValue {
        use prisma::value_container as vc;

        match container.prisma_value.unwrap() {
            vc::PrismaValue::String(v) => PrismaValue::String(v),
            vc::PrismaValue::Float(v) => PrismaValue::Float(v),
            vc::PrismaValue::Boolean(v) => PrismaValue::Boolean(v),
            vc::PrismaValue::DateTime(v) => PrismaValue::DateTime(v.parse::<DateTime<Utc>>().unwrap()),
            vc::PrismaValue::Enum(v) => PrismaValue::Enum(v),
            vc::PrismaValue::Json(v) => PrismaValue::Json(v),
            vc::PrismaValue::Int(v) => PrismaValue::Int(v),
            vc::PrismaValue::Relation(v) => PrismaValue::Relation(v as usize),
            vc::PrismaValue::Null(_) => PrismaValue::Null,
            vc::PrismaValue::Uuid(v) => PrismaValue::Uuid(Uuid::parse_str(&v).unwrap()), // You must die if you didn't send uuid
            vc::PrismaValue::GraphqlId(v) => PrismaValue::GraphqlId(v.into()),
        }
    }
}

impl From<prisma::GraphqlId> for GraphqlId {
    fn from(id: prisma::GraphqlId) -> GraphqlId {
        use prisma::graphql_id as id;

        match id.id_value.unwrap() {
            id::IdValue::String(s) => GraphqlId::String(s),
            id::IdValue::Int(i) => GraphqlId::Int(i as usize),
            id::IdValue::Uuid(s) => GraphqlId::String(s),
        }
    }
}

impl From<PrismaValue> for ValueContainer {
    fn from(pv: PrismaValue) -> ValueContainer {
        use prisma::value_container as vc;

        let prisma_value = match pv {
            PrismaValue::String(v) => vc::PrismaValue::String(v),
            PrismaValue::Float(v) => vc::PrismaValue::Float(v),
            PrismaValue::Boolean(v) => vc::PrismaValue::Boolean(v),
            PrismaValue::DateTime(v) => vc::PrismaValue::DateTime(v.to_rfc3339()),
            PrismaValue::Enum(v) => vc::PrismaValue::Enum(v),
            PrismaValue::Json(v) => vc::PrismaValue::Json(v),
            PrismaValue::Int(v) => vc::PrismaValue::Int(v),
            PrismaValue::Relation(v) => vc::PrismaValue::Relation(v as i64),
            PrismaValue::Null => vc::PrismaValue::Null(true),
            PrismaValue::Uuid(v) => vc::PrismaValue::Uuid(v.to_hyphenated().to_string()),
            PrismaValue::GraphqlId(v) => vc::PrismaValue::GraphqlId(v.into()),
        };

        ValueContainer {
            prisma_value: Some(prisma_value),
        }
    }
}

impl From<GraphqlId> for prisma::GraphqlId {
    fn from(id: GraphqlId) -> prisma::GraphqlId {
        use prisma::graphql_id as id;

        let id_value = match id {
            GraphqlId::String(s) => id::IdValue::String(s),
            GraphqlId::Int(i) => id::IdValue::Int(i as i64),
            GraphqlId::UUID(s) => id::IdValue::Uuid(s.to_hyphenated().to_string()),
        };

        prisma::GraphqlId {
            id_value: Some(id_value),
        }
    }
}

impl From<Node> for prisma::Node {
    fn from(node: Node) -> prisma::Node {
        prisma::Node {
            values: node.values.into_iter().map(ValueContainer::from).collect(),
            parent_id: node.parent_id.map(prisma::GraphqlId::from),
        }
    }
}

impl IntoSelectedFields for prisma::SelectedFields {
    fn into_selected_fields(self, model: ModelRef, from_field: Option<Arc<RelationField>>) -> SelectedFields {
        let fields = self.fields.into_iter().fold(Vec::new(), |mut acc, sf| {
            match sf.field.unwrap() {
                prisma::selected_field::Field::Scalar(field_name) => {
                    let field = model.fields().find_from_scalar(&field_name).unwrap();

                    acc.push(SelectedField::Scalar(SelectedScalarField { field }));
                }
                prisma::selected_field::Field::Relational(rf) => {
                    let field = model.fields().find_from_relation_fields(&rf.field).unwrap();

                    let selected_fields = rf
                        .selected_fields
                        .into_selected_fields(field.related_model(), from_field.clone());

                    acc.push(SelectedField::Relation(SelectedRelationField {
                        field,
                        selected_fields,
                    }));
                }
            }

            acc
        });

        SelectedFields::new(fields, from_field)
    }
}

trait InputValidation {
    fn validate(&self) -> PrismaResult<()>;

    fn validate_args(query_arguments: &crate::protobuf::QueryArguments) -> PrismaResult<()> {
        if let (Some(_), Some(_)) = (query_arguments.first, query_arguments.last) {
            return Err(Error::InvalidConnectionArguments(
                "Cannot have first and last set in the same query",
            ));
        };

        Ok(())
    }
}
