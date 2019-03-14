use crate::protobuf::{prelude::*, InputValidation};
use prisma_common::PrismaResult;

impl InputValidation for GetRelatedNodesInput {
    fn validate(&self) -> PrismaResult<()> {
        Self::validate_args(&self.query_arguments)
    }
}
