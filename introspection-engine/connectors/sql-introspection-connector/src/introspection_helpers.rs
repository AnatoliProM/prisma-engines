use crate::{calculate_datamodel::CalculateDatamodelContext as Context, SqlFamilyTrait};
use psl::{
    datamodel_connector::constraint_names::ConstraintNames,
    dml::{
        Datamodel, FieldArity, FieldType, IndexAlgorithm, IndexDefinition, IndexField, OperatorClass, ScalarField,
        ScalarType, SortOrder,
    },
    parser_database as db,
    schema_ast::ast::WithDocumentation,
    PreviewFeature,
};
use sql::walkers::{ColumnWalker, TableWalker};
use sql_schema_describer::{
    self as sql, mssql::MssqlSchemaExt, postgres::PostgresSchemaExt, ColumnArity, ColumnTypeFamily, IndexType,
    SQLSortOrder, SqlSchema,
};
use std::cmp;
use tracing::debug;

/// This function implements the reverse behaviour of the `Ord` implementation for `Option`: it
/// puts `None` values last, and otherwise orders `Some`s by their contents, like the `Ord` impl.
pub(crate) fn compare_options_none_last<T: cmp::Ord>(a: Option<T>, b: Option<T>) -> cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => cmp::Ordering::Less,
        (None, Some(_)) => cmp::Ordering::Greater,
        (None, None) => cmp::Ordering::Equal,
    }
}

pub(crate) fn is_old_migration_table(table: TableWalker<'_>) -> bool {
    table.name() == "_Migration"
        && table.columns().any(|c| c.name() == "revision")
        && table.columns().any(|c| c.name() == "name")
        && table.columns().any(|c| c.name() == "datamodel")
        && table.columns().any(|c| c.name() == "status")
        && table.columns().any(|c| c.name() == "applied")
        && table.columns().any(|c| c.name() == "rolled_back")
        && table.columns().any(|c| c.name() == "datamodel_steps")
        && table.columns().any(|c| c.name() == "database_migration")
        && table.columns().any(|c| c.name() == "errors")
        && table.columns().any(|c| c.name() == "started_at")
        && table.columns().any(|c| c.name() == "finished_at")
}

pub(crate) fn is_new_migration_table(table: TableWalker<'_>) -> bool {
    table.name() == "_prisma_migrations"
        && table.columns().any(|c| c.name() == "id")
        && table.columns().any(|c| c.name() == "checksum")
        && table.columns().any(|c| c.name() == "finished_at")
        && table.columns().any(|c| c.name() == "migration_name")
        && table.columns().any(|c| c.name() == "logs")
        && table.columns().any(|c| c.name() == "rolled_back_at")
        && table.columns().any(|c| c.name() == "started_at")
        && table.columns().any(|c| c.name() == "applied_steps_count")
}

pub(crate) fn is_relay_table(table: TableWalker<'_>) -> bool {
    table.name() == "_RelayId"
        && table.column("id").is_some()
        && table
            .columns()
            .any(|col| col.name().eq_ignore_ascii_case("stablemodelidentifier"))
}

pub(crate) fn has_created_at_and_updated_at(table: TableWalker<'_>) -> bool {
    let has_created_at = table.columns().any(|col| {
        col.name().eq_ignore_ascii_case("createdat") && col.column_type().family == ColumnTypeFamily::DateTime
    });

    let has_updated_at = table.columns().any(|col| {
        col.name().eq_ignore_ascii_case("updatedat") && col.column_type().family == ColumnTypeFamily::DateTime
    });

    has_created_at && has_updated_at
}

pub(crate) fn is_prisma_join_table(t: TableWalker<'_>) -> bool {
    is_prisma_1_point_0_join_table(t) || is_prisma_1_point_1_or_2_join_table(t)
}

pub(crate) fn is_prisma_1_or_11_list_table(table: TableWalker<'_>) -> bool {
    table.columns().len() == 3
        && table.columns().any(|col| col.name().eq_ignore_ascii_case("nodeid"))
        && table.column("position").is_some()
        && table.column("value").is_some()
}

pub(crate) fn is_prisma_1_point_1_or_2_join_table(table: TableWalker<'_>) -> bool {
    table.name();
    table.columns().len() == 2 && table.indexes().len() >= 2 && common_prisma_m_to_n_relation_conditions(table)
}

pub(crate) fn is_prisma_1_point_0_join_table(table: TableWalker<'_>) -> bool {
    table.columns().len() == 3
        && table.indexes().len() >= 2
        && table.columns().any(|c| c.name() == "id")
        && common_prisma_m_to_n_relation_conditions(table)
}

fn common_prisma_m_to_n_relation_conditions(table: TableWalker<'_>) -> bool {
    fn is_a(column: &str) -> bool {
        column.eq_ignore_ascii_case("a")
    }

    fn is_b(column: &str) -> bool {
        column.eq_ignore_ascii_case("b")
    }

    let mut fks = table.foreign_keys();
    let first_fk = fks.next();
    let second_fk = fks.next();
    let a_b_match = || {
        let first_fk = first_fk.unwrap();
        let second_fk = second_fk.unwrap();
        let first_fk_col = first_fk.constrained_columns().next().unwrap().name();
        let second_fk_col = second_fk.constrained_columns().next().unwrap().name();
        (first_fk.referenced_table().name() <= second_fk.referenced_table().name()
            && is_a(first_fk_col)
            && is_b(second_fk_col))
            || (second_fk.referenced_table().name() <= first_fk.referenced_table().name()
                && is_b(first_fk_col)
                && is_a(second_fk_col))
    };
    table.name().starts_with('_')
        //UNIQUE INDEX [A,B]
        && table.indexes().any(|i| {
            i.columns().len() == 2
                && is_a(i.columns().next().unwrap().as_column().name())
                && is_b(i.columns().nth(1).unwrap().as_column().name())
                && i.is_unique()
        })
    //INDEX [B]
    && table
        .indexes()
        .any(|i| i.columns().len() == 1 && is_b(i.columns().next().unwrap().as_column().name()) && i.index_type() == IndexType::Normal)
        // 2 FKs
        && table.foreign_keys().len() == 2
        // Lexicographically lower model referenced by A
        && a_b_match()
}

//calculators

pub(crate) fn calculate_index(index: sql::walkers::IndexWalker<'_>, ctx: &Context) -> Option<IndexDefinition> {
    let tpe = match index.index_type() {
        IndexType::Unique => psl::dml::IndexType::Unique,
        IndexType::Normal => psl::dml::IndexType::Normal,
        IndexType::Fulltext if ctx.config.preview_features().contains(PreviewFeature::FullTextIndex) => {
            psl::dml::IndexType::Fulltext
        }
        IndexType::Fulltext => psl::dml::IndexType::Normal,
        IndexType::PrimaryKey => return None,
    };

    let default_constraint_name = match index.index_type() {
        IndexType::Unique => {
            let columns = index.column_names().collect::<Vec<_>>();
            ConstraintNames::unique_index_name(index.table().name(), &columns, ctx.active_connector())
        }
        _ => {
            let columns = index.column_names().collect::<Vec<_>>();
            ConstraintNames::non_unique_index_name(index.table().name(), &columns, ctx.active_connector())
        }
    };

    let db_name = if index.name() == default_constraint_name {
        None
    } else {
        Some(index.name().to_owned())
    };

    Some(IndexDefinition {
        name: None,
        db_name,
        fields: index
            .columns()
            .map(|c| {
                let sort_order = c.sort_order().and_then(|sort| match sort {
                    SQLSortOrder::Asc => None,
                    SQLSortOrder::Desc => Some(SortOrder::Desc),
                });

                let operator_class = get_opclass(c.id, index.schema, ctx);

                IndexField {
                    path: vec![(ctx.column_prisma_name(c.as_column().id).to_owned(), None)],
                    sort_order,
                    length: c.length(),
                    operator_class,
                }
            })
            .collect(),
        tpe,
        defined_on_field: index.columns().len() == 1,
        algorithm: index_algorithm(index, ctx),
        clustered: index_is_clustered(index.id, index.schema, ctx),
    })
}

pub(crate) fn calculate_scalar_field(
    column: ColumnWalker<'_>,
    remapped_fields: &mut Vec<crate::warnings::ModelAndField>,
    ctx: &mut Context<'_>,
) -> ScalarField {
    let existing_field = ctx.existing_scalar_field(column.id);

    if let Some(field) = existing_field.filter(|f| f.mapped_name().is_some()) {
        remapped_fields.push(crate::warnings::ModelAndField {
            model: field.model().name().to_owned(),
            field: field.name().to_owned(),
        });
    }

    let arity = match column.column_type().arity {
        _ if column.is_single_primary_key() && column.is_autoincrement() => FieldArity::Required,
        ColumnArity::Required => FieldArity::Required,
        ColumnArity::Nullable => FieldArity::Optional,
        ColumnArity::List => FieldArity::List,
    };

    let mut default_value = crate::defaults::calculate_default(column, ctx);

    let default_default_value =
        ConstraintNames::default_name(column.table().name(), column.name(), ctx.active_connector());

    if let Some(ref mut default_value) = default_value {
        if default_value.db_name == Some(default_default_value) {
            default_value.db_name = None;
        }
    }

    ScalarField {
        name: existing_field
            .map(|f| f.name())
            .unwrap_or_else(|| column.name())
            .to_owned(),
        arity,
        field_type: calculate_scalar_field_type_with_native_types(column, ctx),
        database_name: existing_field.and_then(|f| f.mapped_name()).map(ToOwned::to_owned),
        documentation: existing_field
            .and_then(|f| f.ast_field().documentation())
            .map(ToOwned::to_owned),
        default_value,
        is_generated: false,
        is_commented_out: false,
        is_updated_at: existing_field.map(|f| f.is_updated_at()).unwrap_or(false),
        is_ignored: existing_field.map(|f| f.is_ignored()).unwrap_or(false),
    }
}

pub(crate) fn calculate_scalar_field_type_for_native_type(column: ColumnWalker<'_>, ctx: &Context) -> FieldType {
    debug!("Calculating field type for '{}'", column.name());
    let fdt = column.column_type().full_data_type.to_owned();

    match column.column_type_family() {
        ColumnTypeFamily::Int => FieldType::Scalar(ScalarType::Int, None),
        ColumnTypeFamily::BigInt => FieldType::Scalar(ScalarType::BigInt, None),
        ColumnTypeFamily::Float => FieldType::Scalar(ScalarType::Float, None),
        ColumnTypeFamily::Decimal => FieldType::Scalar(ScalarType::Decimal, None),
        ColumnTypeFamily::Boolean => FieldType::Scalar(ScalarType::Boolean, None),
        ColumnTypeFamily::String => FieldType::Scalar(ScalarType::String, None),
        ColumnTypeFamily::DateTime => FieldType::Scalar(ScalarType::DateTime, None),
        ColumnTypeFamily::Json => FieldType::Scalar(ScalarType::Json, None),
        ColumnTypeFamily::Uuid => FieldType::Scalar(ScalarType::String, None),
        ColumnTypeFamily::Binary => FieldType::Scalar(ScalarType::Bytes, None),
        ColumnTypeFamily::Enum(id) => FieldType::Enum(ctx.enum_prisma_name(*id).to_owned()),
        ColumnTypeFamily::Unsupported(_) => FieldType::Unsupported(fdt),
    }
}

pub(crate) fn calculate_scalar_field_type_with_native_types(column: sql::ColumnWalker<'_>, ctx: &Context) -> FieldType {
    debug!("Calculating native field type for '{}'", column.name());
    let scalar_type = calculate_scalar_field_type_for_native_type(column, ctx);

    match scalar_type {
        FieldType::Scalar(scal_type, _) => match &column.column_type().native_type {
            None => scalar_type,
            Some(native_type) => {
                let is_default = ctx.active_connector().native_type_is_default_for_scalar_type(
                    native_type,
                    &dml_scalar_type_to_parser_database_scalar_type(scal_type),
                );

                if is_default {
                    FieldType::Scalar(scal_type, None)
                } else {
                    let instance = psl::dml::NativeTypeInstance::new(native_type.clone(), ctx.active_connector());

                    FieldType::Scalar(scal_type, Some(instance))
                }
            }
        },
        field_type => field_type,
    }
}

fn dml_scalar_type_to_parser_database_scalar_type(st: ScalarType) -> db::ScalarType {
    match st {
        ScalarType::Int => db::ScalarType::Int,
        ScalarType::BigInt => db::ScalarType::BigInt,
        ScalarType::Float => db::ScalarType::Float,
        ScalarType::Boolean => db::ScalarType::Boolean,
        ScalarType::String => db::ScalarType::String,
        ScalarType::DateTime => db::ScalarType::DateTime,
        ScalarType::Json => db::ScalarType::Json,
        ScalarType::Bytes => db::ScalarType::Bytes,
        ScalarType::Decimal => db::ScalarType::Decimal,
    }
}

// misc

pub(crate) fn deduplicate_relation_field_names(datamodel: &mut Datamodel) {
    let mut duplicated_relation_fields = vec![];

    for model in datamodel.models() {
        for field in model.relation_fields() {
            if model.fields().filter(|f| field.name == f.name()).count() > 1 {
                duplicated_relation_fields.push((
                    model.name.clone(),
                    field.name.clone(),
                    field.relation_info.name.clone(),
                ));
            }
        }
    }

    duplicated_relation_fields
        .iter()
        .for_each(|(model, field, relation_name)| {
            let mut field = datamodel.find_model_mut(model).find_relation_field_mut(field);
            //todo self vs normal relation?
            field.name = format!("{}_{}", field.name, &relation_name);
        });
}

fn index_algorithm(index: sql::walkers::IndexWalker<'_>, ctx: &Context) -> Option<IndexAlgorithm> {
    if !ctx.sql_family().is_postgres() {
        return None;
    }

    let data: &PostgresSchemaExt = index.schema.downcast_connector_data();

    match data.index_algorithm(index.id) {
        sql::postgres::SqlIndexAlgorithm::BTree => None,
        sql::postgres::SqlIndexAlgorithm::Hash => Some(IndexAlgorithm::Hash),
        sql::postgres::SqlIndexAlgorithm::Gist => Some(IndexAlgorithm::Gist),
        sql::postgres::SqlIndexAlgorithm::Gin => Some(IndexAlgorithm::Gin),
        sql::postgres::SqlIndexAlgorithm::SpGist => Some(IndexAlgorithm::SpGist),
        sql::postgres::SqlIndexAlgorithm::Brin => Some(IndexAlgorithm::Brin),
    }
}

fn index_is_clustered(index_id: sql::IndexId, schema: &SqlSchema, ctx: &Context) -> Option<bool> {
    if !ctx.sql_family().is_mssql() {
        return None;
    }

    let ext: &MssqlSchemaExt = schema.downcast_connector_data();
    let clustered = ext.index_is_clustered(index_id);

    if !clustered {
        return None;
    }

    Some(clustered)
}

pub(crate) fn primary_key_is_clustered(pkid: sql::IndexId, ctx: &Context) -> Option<bool> {
    if !ctx.sql_family().is_mssql() {
        return None;
    }

    let ext: &MssqlSchemaExt = ctx.schema.downcast_connector_data();

    let clustered = ext.index_is_clustered(pkid);

    if clustered {
        return None;
    }

    Some(clustered)
}

fn get_opclass(index_field_id: sql::IndexColumnId, schema: &SqlSchema, ctx: &Context) -> Option<OperatorClass> {
    if !ctx.sql_family().is_postgres() {
        return None;
    }

    let ext: &PostgresSchemaExt = schema.downcast_connector_data();

    let opclass = match ext.get_opclass(index_field_id) {
        Some(opclass) => opclass,
        None => return None,
    };

    match &opclass.kind {
        _ if opclass.is_default => None,
        sql::postgres::SQLOperatorClassKind::InetOps => Some(OperatorClass::InetOps),
        sql::postgres::SQLOperatorClassKind::JsonbOps => Some(OperatorClass::JsonbOps),
        sql::postgres::SQLOperatorClassKind::JsonbPathOps => Some(OperatorClass::JsonbPathOps),
        sql::postgres::SQLOperatorClassKind::ArrayOps => Some(OperatorClass::ArrayOps),
        sql::postgres::SQLOperatorClassKind::TextOps => Some(OperatorClass::TextOps),
        sql::postgres::SQLOperatorClassKind::BitMinMaxOps => Some(OperatorClass::BitMinMaxOps),
        sql::postgres::SQLOperatorClassKind::VarBitMinMaxOps => Some(OperatorClass::VarBitMinMaxOps),
        sql::postgres::SQLOperatorClassKind::BpcharBloomOps => Some(OperatorClass::BpcharBloomOps),
        sql::postgres::SQLOperatorClassKind::BpcharMinMaxOps => Some(OperatorClass::BpcharMinMaxOps),
        sql::postgres::SQLOperatorClassKind::ByteaBloomOps => Some(OperatorClass::ByteaBloomOps),
        sql::postgres::SQLOperatorClassKind::ByteaMinMaxOps => Some(OperatorClass::ByteaMinMaxOps),
        sql::postgres::SQLOperatorClassKind::DateBloomOps => Some(OperatorClass::DateBloomOps),
        sql::postgres::SQLOperatorClassKind::DateMinMaxOps => Some(OperatorClass::DateMinMaxOps),
        sql::postgres::SQLOperatorClassKind::DateMinMaxMultiOps => Some(OperatorClass::DateMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::Float4BloomOps => Some(OperatorClass::Float4BloomOps),
        sql::postgres::SQLOperatorClassKind::Float4MinMaxOps => Some(OperatorClass::Float4MinMaxOps),
        sql::postgres::SQLOperatorClassKind::Float4MinMaxMultiOps => Some(OperatorClass::Float4MinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::Float8BloomOps => Some(OperatorClass::Float8BloomOps),
        sql::postgres::SQLOperatorClassKind::Float8MinMaxOps => Some(OperatorClass::Float8MinMaxOps),
        sql::postgres::SQLOperatorClassKind::Float8MinMaxMultiOps => Some(OperatorClass::Float8MinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::InetInclusionOps => Some(OperatorClass::InetInclusionOps),
        sql::postgres::SQLOperatorClassKind::InetBloomOps => Some(OperatorClass::InetBloomOps),
        sql::postgres::SQLOperatorClassKind::InetMinMaxOps => Some(OperatorClass::InetMinMaxOps),
        sql::postgres::SQLOperatorClassKind::InetMinMaxMultiOps => Some(OperatorClass::InetMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::Int2BloomOps => Some(OperatorClass::Int2BloomOps),
        sql::postgres::SQLOperatorClassKind::Int2MinMaxOps => Some(OperatorClass::Int2MinMaxOps),
        sql::postgres::SQLOperatorClassKind::Int2MinMaxMultiOps => Some(OperatorClass::Int2MinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::Int4BloomOps => Some(OperatorClass::Int4BloomOps),
        sql::postgres::SQLOperatorClassKind::Int4MinMaxOps => Some(OperatorClass::Int4MinMaxOps),
        sql::postgres::SQLOperatorClassKind::Int4MinMaxMultiOps => Some(OperatorClass::Int4MinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::Int8BloomOps => Some(OperatorClass::Int8BloomOps),
        sql::postgres::SQLOperatorClassKind::Int8MinMaxOps => Some(OperatorClass::Int8MinMaxOps),
        sql::postgres::SQLOperatorClassKind::Int8MinMaxMultiOps => Some(OperatorClass::Int8MinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::NumericBloomOps => Some(OperatorClass::NumericBloomOps),
        sql::postgres::SQLOperatorClassKind::NumericMinMaxOps => Some(OperatorClass::NumericMinMaxOps),
        sql::postgres::SQLOperatorClassKind::NumericMinMaxMultiOps => Some(OperatorClass::NumericMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::OidBloomOps => Some(OperatorClass::OidBloomOps),
        sql::postgres::SQLOperatorClassKind::OidMinMaxOps => Some(OperatorClass::OidMinMaxOps),
        sql::postgres::SQLOperatorClassKind::OidMinMaxMultiOps => Some(OperatorClass::OidMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::TextBloomOps => Some(OperatorClass::TextBloomOps),
        sql::postgres::SQLOperatorClassKind::TextMinMaxOps => Some(OperatorClass::TextMinMaxOps),
        sql::postgres::SQLOperatorClassKind::TimestampBloomOps => Some(OperatorClass::TimestampBloomOps),
        sql::postgres::SQLOperatorClassKind::TimestampMinMaxOps => Some(OperatorClass::TimestampMinMaxOps),
        sql::postgres::SQLOperatorClassKind::TimestampMinMaxMultiOps => Some(OperatorClass::TimestampMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::TimestampTzBloomOps => Some(OperatorClass::TimestampTzBloomOps),
        sql::postgres::SQLOperatorClassKind::TimestampTzMinMaxOps => Some(OperatorClass::TimestampTzMinMaxOps),
        sql::postgres::SQLOperatorClassKind::TimestampTzMinMaxMultiOps => {
            Some(OperatorClass::TimestampTzMinMaxMultiOps)
        }
        sql::postgres::SQLOperatorClassKind::TimeBloomOps => Some(OperatorClass::TimeBloomOps),
        sql::postgres::SQLOperatorClassKind::TimeMinMaxOps => Some(OperatorClass::TimeMinMaxOps),
        sql::postgres::SQLOperatorClassKind::TimeMinMaxMultiOps => Some(OperatorClass::TimeMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::TimeTzBloomOps => Some(OperatorClass::TimeTzBloomOps),
        sql::postgres::SQLOperatorClassKind::TimeTzMinMaxOps => Some(OperatorClass::TimeTzMinMaxOps),
        sql::postgres::SQLOperatorClassKind::TimeTzMinMaxMultiOps => Some(OperatorClass::TimeTzMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::UuidBloomOps => Some(OperatorClass::UuidBloomOps),
        sql::postgres::SQLOperatorClassKind::UuidMinMaxOps => Some(OperatorClass::UuidMinMaxOps),
        sql::postgres::SQLOperatorClassKind::UuidMinMaxMultiOps => Some(OperatorClass::UuidMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::Raw(c) => Some(OperatorClass::Raw(c.to_string().into())),
    }
}
