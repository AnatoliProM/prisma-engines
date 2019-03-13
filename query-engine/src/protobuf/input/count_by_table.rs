use crate::protobuf::{prelude::*, InputValidation};
use prisma_common::PrismaResult;

impl InputValidation for CountByTableInput {
    fn validate(&self) -> PrismaResult<()> {
        Ok(())
    }
}
