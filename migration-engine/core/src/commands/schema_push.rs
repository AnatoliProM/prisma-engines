use crate::{json_rpc::types::*, parse_schema, CoreResult};
use migration_connector::{ConnectorError, DiffTarget, MigrationConnector};

/// Command to bring the local database in sync with the prisma schema, without
/// interacting with the migrations directory nor the migrations table.
pub async fn schema_push(
    input: SchemaPushInput,
    connector: &mut dyn MigrationConnector,
) -> CoreResult<SchemaPushOutput> {
    let datamodel = parse_schema(&input.schema)?;

    if let Some(err) = connector.check_database_version_compatibility(&datamodel) {
        return Err(ConnectorError::user_facing(err));
    };

    let from = connector.database_schema_from_diff_target(DiffTarget::Database).await?;
    let to = connector
        .database_schema_from_diff_target(DiffTarget::Datamodel(&input.schema))
        .await?;
    let database_migration = connector.diff(from, to)?;

    let checks = connector
        .destructive_change_checker()
        .check(&database_migration)
        .await?;

    let executed_steps = match (checks.unexecutable_migrations.len(), checks.warnings.len(), input.force) {
        (unexecutable, _, _) if unexecutable > 0 => {
            tracing::warn!(unexecutable = ?checks.unexecutable_migrations, "Aborting migration because at least one unexecutable step was detected.");

            0
        }
        (0, 0, _) | (0, _, true) => connector.apply_migration(&database_migration).await?,
        _ => {
            tracing::info!(
                "The migration was not applied because it triggered warnings and the force flag was not passed."
            );

            0
        }
    };

    let warnings = checks.warnings.into_iter().map(|warning| warning.description).collect();

    let unexecutable = checks
        .unexecutable_migrations
        .into_iter()
        .map(|unexecutable| unexecutable.description)
        .collect();

    Ok(SchemaPushOutput {
        executed_steps,
        warnings,
        unexecutable,
    })
}
