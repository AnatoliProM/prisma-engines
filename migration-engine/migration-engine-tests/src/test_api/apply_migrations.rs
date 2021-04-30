use migration_core::{commands::ApplyMigrationsInput, commands::ApplyMigrationsOutput, CoreResult, GenericApi};
use tempfile::TempDir;

use crate::AssertionResult;

#[must_use = "This struct does nothing on its own. See ApplyMigrations::send()"]
pub struct ApplyMigrations<'a> {
    api: &'a dyn GenericApi,
    migrations_directory: &'a TempDir,
    rt: Option<&'a tokio::runtime::Runtime>,
}

impl<'a> ApplyMigrations<'a> {
    pub fn new(api: &'a dyn GenericApi, migrations_directory: &'a TempDir) -> Self {
        ApplyMigrations {
            api,
            migrations_directory,
            rt: None,
        }
    }

    pub fn new_sync(
        api: &'a dyn GenericApi,
        migrations_directory: &'a TempDir,
        rt: &'a tokio::runtime::Runtime,
    ) -> Self {
        ApplyMigrations {
            api,
            migrations_directory,
            rt: Some(rt),
        }
    }

    pub async fn send(self) -> CoreResult<ApplyMigrationsAssertion<'a>> {
        let output = self
            .api
            .apply_migrations(&ApplyMigrationsInput {
                migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
            })
            .await?;

        Ok(ApplyMigrationsAssertion {
            output,
            _api: self.api,
            _migrations_directory: self.migrations_directory,
        })
    }

    pub fn send_sync(self) -> CoreResult<ApplyMigrationsAssertion<'a>> {
        self.rt.unwrap().block_on(self.send())
    }
}

pub struct ApplyMigrationsAssertion<'a> {
    output: ApplyMigrationsOutput,
    _api: &'a dyn GenericApi,
    _migrations_directory: &'a TempDir,
}

impl std::fmt::Debug for ApplyMigrationsAssertion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ApplyMigrationsAssertion {{ .. }}")
    }
}

impl<'a> ApplyMigrationsAssertion<'a> {
    pub fn assert_applied_migrations(self, names: &[&str]) -> AssertionResult<Self> {
        let found_names: Vec<&str> = self
            .output
            .applied_migration_names
            .iter()
            .map(|name| &name[15..])
            .collect();

        anyhow::ensure!(
            found_names == names,
            "Assertion failed. The applied migrations do not match the expectations. ({:?} vs {:?})",
            found_names,
            names
        );

        Ok(self)
    }

    pub fn into_output(self) -> ApplyMigrationsOutput {
        self.output
    }
}
