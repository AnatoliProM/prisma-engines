use crate::{ConnectorResult, MigrationStep};
use datamodel::Datamodel;

pub trait DatabaseMigrationInferrer<T>: Send + Sync + 'static {
    fn infer(&self, previous: &Datamodel, next: &Datamodel, steps: &Vec<MigrationStep>) -> ConnectorResult<T>;
}
