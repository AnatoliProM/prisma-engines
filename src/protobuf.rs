mod envelope;
mod get_node_by_where_input;
mod interface;

pub use envelope::ProtoBufEnvelope;
pub use get_node_by_where_input::*;
pub use interface::ProtoBufInterface;

pub mod prisma {
    include!(concat!(env!("OUT_DIR"), "/prisma.rs"));
}

use prisma::{
    result, rpc_response as rpc, selected_field, Error as ProtoError, Header, NodesResult, Result,
    RpcResponse, SelectedField,
};

use crate::Error as CrateError;

impl RpcResponse {
    pub fn header() -> Header {
        Header {
            type_name: String::from("RpcResponse"),
        }
    }

    pub fn empty() -> RpcResponse {
        RpcResponse {
            header: Self::header(),
            response: Some(rpc::Response::Result(Result { value: None })),
        }
    }

    pub fn ok(result: NodesResult) -> RpcResponse {
        RpcResponse {
            header: Self::header(),
            response: Some(rpc::Response::Result(prisma::Result {
                value: Some(result::Value::NodesResult(result)),
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

impl SelectedField {
    pub fn is_scalar(&self) -> bool {
        match self.field {
            Some(selected_field::Field::Scalar(_)) => true,
            _ => false,
        }
    }

    pub fn is_relational(&self) -> bool {
        match self.field {
            Some(selected_field::Field::Relational { .. }) => true,
            _ => false,
        }
    }
}
