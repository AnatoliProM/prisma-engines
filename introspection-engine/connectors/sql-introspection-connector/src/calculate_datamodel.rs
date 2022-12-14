use crate::{introspection::introspect, SqlFamilyTrait, SqlIntrospectionResult};
use introspection_connector::{IntrospectionContext, IntrospectionResult, Warning};
use psl::{
    builtin_connectors::*, datamodel_connector::Connector, dml::Datamodel, parser_database::walkers, Configuration,
};
use quaint::prelude::SqlFamily;
use sql_schema_describer as sql;

pub(crate) struct CalculateDatamodelContext<'a> {
    pub(crate) config: &'a Configuration,
    pub(crate) render_config: bool,
    pub(crate) previous_datamodel: &'a Datamodel,
    pub(crate) schema: &'a sql::SqlSchema,
    pub(crate) sql_family: SqlFamily,
    pub(crate) warnings: &'a mut Vec<Warning>,
    pub(crate) previous_schema: &'a psl::ValidatedSchema,
    introspection_map: crate::introspection_map::IntrospectionMap,
}

impl<'a> CalculateDatamodelContext<'a> {
    pub(crate) fn is_cockroach(&self) -> bool {
        self.active_connector().provider_name() == COCKROACH.provider_name()
    }

    pub(crate) fn foreign_keys_enabled(&self) -> bool {
        self.config
            .datasources
            .first()
            .unwrap()
            .relation_mode()
            .uses_foreign_keys()
    }

    pub(crate) fn active_connector(&self) -> &'static dyn Connector {
        self.config.datasources.first().unwrap().active_connector
    }

    /// Given a SQL enum from the database, this method returns the enum that matches it (by name)
    /// in the Prisma schema.
    pub(crate) fn existing_enum(&self, id: sql::EnumId) -> Option<walkers::EnumWalker<'a>> {
        self.introspection_map
            .existing_enums
            .get(&id)
            .map(|id| self.previous_schema.db.walk(*id))
    }

    /// Given a SQL enum from the database, this method returns the name it will be given in the
    /// introspected schema. If it matches a remapped enum in the Prisma schema, it is taken into
    /// account.
    pub(crate) fn enum_prisma_name(&self, id: sql::EnumId) -> &'a str {
        self.existing_enum(id)
            .map(|enm| enm.name())
            .unwrap_or_else(|| self.schema.walk(id).name())
    }

    /// Given a foreign key from the database, this methods returns the existing relation in the
    /// Prisma schema that matches it.
    pub(crate) fn existing_inline_relation(&self, id: sql::ForeignKeyId) -> Option<walkers::InlineRelationWalker<'a>> {
        self.introspection_map
            .existing_inline_relations
            .get(&id)
            .map(|relation_id| self.previous_schema.db.walk(*relation_id).refine().as_inline().unwrap())
    }

    pub(crate) fn existing_m2m_relation(
        &self,
        id: sql::TableId,
    ) -> Option<walkers::ImplicitManyToManyRelationWalker<'a>> {
        self.introspection_map
            .existing_m2m_relations
            .get(&id)
            .map(|relation_id| self.previous_schema.db.walk(*relation_id))
    }

    pub(crate) fn existing_model(&self, id: sql::TableId) -> Option<walkers::ModelWalker<'a>> {
        self.introspection_map
            .existing_models
            .get(&id)
            .map(|id| self.previous_schema.db.walk(*id))
    }

    pub(crate) fn existing_scalar_field(&self, id: sql::ColumnId) -> Option<walkers::ScalarFieldWalker<'a>> {
        self.introspection_map
            .existing_scalar_fields
            .get(&id)
            .map(|(model_id, field_id)| self.previous_schema.db.walk(*model_id).scalar_field(*field_id))
    }

    pub(crate) fn column_prisma_name(&self, id: sql::ColumnId) -> &'a str {
        self.existing_scalar_field(id)
            .map(|sf| sf.name())
            .unwrap_or_else(|| self.schema.walk(id).name())
    }

    pub(crate) fn model_prisma_name(&self, id: sql::TableId) -> &'a str {
        self.existing_model(id)
            .map(|model| model.name())
            .unwrap_or_else(|| self.schema.walk(id).name())
    }
}

/// Calculate a data model from a database schema.
pub fn calculate_datamodel(
    schema: &sql::SqlSchema,
    ctx: &IntrospectionContext,
) -> SqlIntrospectionResult<IntrospectionResult> {
    let mut warnings = Vec::new();

    let mut context = CalculateDatamodelContext {
        config: ctx.configuration(),
        render_config: ctx.render_config,
        previous_datamodel: &ctx.previous_data_model,
        schema,
        sql_family: ctx.sql_family(),
        previous_schema: ctx.previous_schema(),
        warnings: &mut warnings,
        introspection_map: crate::introspection_map::IntrospectionMap::new(schema, ctx.previous_schema()),
    };

    let (version, data_model, is_empty) = introspect(&mut context)?;

    Ok(IntrospectionResult {
        data_model,
        is_empty,
        warnings,
        version,
    })
}
