use super::list_migrations::ListMigrationStepsOutput;
use crate::commands::command::{MigrationCommand, CommandResult};
use crate::migration_engine::MigrationEngine;
use migration_connector::*;

pub struct UnapplyMigrationCommand {
    input: UnapplyMigrationInput,
}
#[allow(unused)]
impl MigrationCommand for UnapplyMigrationCommand {
    type Input = UnapplyMigrationInput;
    type Output = UnapplyMigrationOutput;

    fn new(input: Self::Input) -> Box<Self> {
        Box::new(UnapplyMigrationCommand { input })
    }

    fn execute(&self, engine: &Box<MigrationEngine>) -> CommandResult<Self::Output> {
        println!("{:?}", self.input);
        let connector = engine.connector();
        let result = match connector.migration_persistence().last() {
            None => UnapplyMigrationOutput {
                rolled_back: ListMigrationStepsOutput {
                    id: "foo".to_string(),
                    steps: Vec::new(),
                    status: MigrationStatus::Pending,
                    datamodel: "".to_string(),
                },
                active: ListMigrationStepsOutput {
                    id: "bar".to_string(),
                    steps: Vec::new(),
                    status: MigrationStatus::Pending,
                    datamodel: "".to_string(),
                },
                errors: vec!["There is no last migration that can be rolled back.".to_string()],
            },
            Some(migration_to_rollback) => {
                let database_migration =
                    connector.deserialize_database_migration(migration_to_rollback.database_migration.clone());
                connector
                    .migration_applier()
                    .unapply(&migration_to_rollback, &database_migration);

                let new_active_migration = match connector.migration_persistence().last() {
                    Some(m) => m,
                    None => Migration::new("no-migration".to_string()),
                };

                UnapplyMigrationOutput {
                    rolled_back: ListMigrationStepsOutput {
                        id: migration_to_rollback.name,
                        steps: migration_to_rollback.datamodel_steps,
                        status: migration_to_rollback.status,
                        datamodel: engine.render_datamodel(&migration_to_rollback.datamodel),
                    },
                    active: ListMigrationStepsOutput {
                        id: new_active_migration.name,
                        steps: new_active_migration.datamodel_steps,
                        status: new_active_migration.status,
                        datamodel: engine.render_datamodel(&migration_to_rollback.datamodel),
                    },
                    errors: Vec::new(),
                }
            }
        };
        Ok(result)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnapplyMigrationInput {
    pub project_info: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnapplyMigrationOutput {
    pub rolled_back: ListMigrationStepsOutput,
    pub active: ListMigrationStepsOutput,
    pub errors: Vec<String>,
}
