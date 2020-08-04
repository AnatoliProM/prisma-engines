use crate::{sql_renderer::IteratorJoin, DatabaseInfo};
use datamodel::{
    walkers::{walk_models, walk_relations, walk_scalar_fields, ModelWalker, ScalarFieldWalker, TypeWalker},
    Datamodel, DefaultValue, FieldArity, IndexDefinition, IndexType, ScalarType, ValueGenerator, ValueGeneratorFn,
    WithDatabaseName,
};
use prisma_value::PrismaValue;
use quaint::prelude::SqlFamily;
use sql_schema_describer::{self as sql, ColumnArity};

pub struct SqlSchemaCalculator<'a> {
    data_model: &'a Datamodel,
    database_info: &'a DatabaseInfo,
}

impl<'a> SqlSchemaCalculator<'a> {
    pub fn calculate(data_model: &Datamodel, database_info: &DatabaseInfo) -> sql::SqlSchema {
        let calculator = SqlSchemaCalculator {
            data_model,
            database_info,
        };
        calculator.calculate_internal()
    }

    fn calculate_internal(&self) -> sql::SqlSchema {
        let mut tables = Vec::with_capacity(self.data_model.models().len());
        let model_tables_without_inline_relations = self.calculate_model_tables();

        for (model, mut table) in model_tables_without_inline_relations {
            self.add_inline_relations_to_model_tables(model, &mut table);
            tables.push(table);
        }

        tables.extend(self.calculate_relation_tables());

        let enums = self.calculate_enums();
        let sequences = Vec::new();

        sql::SqlSchema {
            tables,
            enums,
            sequences,
        }
    }

    fn calculate_enums(&self) -> Vec<sql::Enum> {
        match self.database_info.sql_family() {
            SqlFamily::Postgres => self
                .data_model
                .enums()
                .map(|r#enum| sql::Enum {
                    name: r#enum.final_database_name().to_owned(),
                    values: r#enum.database_values(),
                })
                .collect(),
            SqlFamily::Mysql => {
                // This is a lower bound for the size of the generated enums (we assume each enum is
                // used at least once).
                let mut enums = Vec::with_capacity(self.data_model.enums().len());

                let enum_fields = walk_scalar_fields(&self.data_model)
                    .filter_map(|field| field.field_type().as_enum().map(|enum_ref| (field, enum_ref)));

                for (field, enum_tpe) in enum_fields {
                    let sql_enum = sql::Enum {
                        name: format!(
                            "{model_name}_{field_name}",
                            model_name = field.model().database_name(),
                            field_name = field.db_name()
                        ),
                        values: enum_tpe.r#enum.database_values(),
                    };

                    enums.push(sql_enum)
                }

                enums
            }
            _ => Vec::new(),
        }
    }

    fn calculate_model_tables<'iter>(&'iter self) -> impl Iterator<Item = (ModelWalker<'a>, sql::Table)> + 'iter {
        walk_models(self.data_model).map(move |model| {
            let columns = model
                .scalar_fields()
                .flat_map(|f| match f.field_type() {
                    TypeWalker::Base(_) => Some(sql::Column {
                        name: f.db_name().to_owned(),
                        tpe: column_type(&f),
                        default: migration_value_new(&f),
                        auto_increment: matches!(f.default_value(), Some(DefaultValue::Expression(ValueGenerator { generator: ValueGeneratorFn::Autoincrement, .. }))),
                    }),
                    TypeWalker::Enum(r#enum) => {
                        let enum_db_name = r#enum.db_name();
                        Some(sql::Column {
                            name: f.db_name().to_owned(),
                            tpe: enum_column_type(&f, &self.database_info, enum_db_name),
                            default: migration_value_new(&f),
                            auto_increment: false,
                        })
                    }
                    _ => None,
                })
                .collect();

            let primary_key = Some(sql::PrimaryKey {
                columns: model
                    .id_fields()
                    .map(|field| field.db_name().to_owned())
                    .collect(),
                sequence: None,
                constraint_name: None,
            }).filter(|pk| !pk.columns.is_empty());

            let single_field_indexes = model.scalar_fields().filter(|f| f.is_unique()).map(|f|
                sql::Index {
                    name: format!("{}.{}_unique", &model.db_name(), &f.db_name()),
                    columns: vec![f.db_name().to_owned()],
                    tpe: sql::IndexType::Unique,
                }
            );

            let multiple_field_indexes = model.indexes().map(|index_definition: &IndexDefinition| {
                let referenced_fields: Vec<ScalarFieldWalker<'_>> = index_definition
                    .fields
                    .iter()
                    .map(|field_name| model.find_scalar_field(field_name).expect("Unknown field in index directive."))
                    .collect();

                let index_type = match index_definition.tpe {
                    IndexType::Unique => sql::IndexType::Unique,
                    IndexType::Normal => sql::IndexType::Normal,
                };

                let index_name = index_definition.name.clone().unwrap_or_else(|| {
                    format!(
                        "{table}.{fields}_{qualifier}",
                        table = &model.db_name(),
                        fields = referenced_fields.iter().map(|field| field.db_name()).join("_"),
                        qualifier = if index_type.is_unique() { "unique" } else { "index" },
                    )
                });

                sql::Index {
                    name: index_name,
                    // The model index definition uses the model field names, but the SQL Index
                    // wants the column names.
                    columns: referenced_fields
                        .iter()
                        .map(|field| field.db_name().to_owned())
                        .collect(),
                    tpe: index_type,
                }
            });

            let table = sql::Table {
                name: model.database_name().to_owned(),
                columns,
                indices: single_field_indexes.chain(multiple_field_indexes).collect(),
                primary_key,
                foreign_keys: Vec::new(),
            };

            (model, table)
        })
    }

    fn add_inline_relations_to_model_tables(&self, model: ModelWalker<'a>, table: &mut sql::Table) {
        let relation_fields = model
            .relation_fields()
            .filter(|relation_field| !relation_field.is_virtual());

        for relation_field in relation_fields {
            let fk_columns: Vec<String> = relation_field.referencing_columns().map(String::from).collect();

            // Optional unique index for 1:1 relations.
            if relation_field.is_one_to_one() {
                add_one_to_one_relation_unique_index(table, &fk_columns);
            }

            // Foreign key
            {
                let fk = sql::ForeignKey {
                    constraint_name: None,
                    columns: fk_columns,
                    referenced_table: relation_field.referenced_table_name().to_owned(),
                    referenced_columns: relation_field.referenced_columns().map(String::from).collect(),
                    on_delete_action: match column_arity(relation_field.arity()) {
                        ColumnArity::Required => sql::ForeignKeyAction::Cascade,
                        _ => sql::ForeignKeyAction::SetNull,
                    },
                };

                table.foreign_keys.push(fk);
            }
        }
    }

    fn calculate_relation_tables<'b>(&'b self) -> impl Iterator<Item = sql::Table> + 'b {
        walk_relations(self.data_model)
            .filter_map(|relation| relation.as_m2m())
            .map(|m2m| {
                let table_name = m2m.table_name();
                let model_a_id = m2m.model_a_id();
                let model_b_id = m2m.model_b_id();
                let model_a = model_a_id.model();
                let model_b = model_b_id.model();

                let foreign_keys = vec![
                    sql::ForeignKey {
                        constraint_name: None,
                        columns: vec![m2m.model_a_column().into()],
                        referenced_table: model_a.db_name().into(),
                        referenced_columns: vec![model_a_id.db_name().into()],
                        on_delete_action: sql::ForeignKeyAction::Cascade,
                    },
                    sql::ForeignKey {
                        constraint_name: None,
                        columns: vec![m2m.model_b_column().into()],
                        referenced_table: model_b.db_name().into(),
                        referenced_columns: vec![model_b_id.db_name().into()],
                        on_delete_action: sql::ForeignKeyAction::Cascade,
                    },
                ];

                let indexes = vec![
                    sql::Index {
                        name: format!("{}_AB_unique", &table_name),
                        columns: vec![m2m.model_a_column().into(), m2m.model_b_column().into()],
                        tpe: sql::IndexType::Unique,
                    },
                    sql::Index {
                        name: format!("{}_B_index", &table_name),
                        columns: vec![m2m.model_b_column().into()],
                        tpe: sql::IndexType::Normal,
                    },
                ];

                let columns = vec![
                    sql::Column {
                        name: m2m.model_a_column().into(),
                        tpe: column_type(&model_a_id),
                        default: None,
                        auto_increment: false,
                    },
                    sql::Column {
                        name: m2m.model_b_column().into(),
                        tpe: column_type(&model_b_id),
                        default: None,
                        auto_increment: false,
                    },
                ];

                sql::Table {
                    name: table_name,
                    columns,
                    indices: indexes,
                    primary_key: None,
                    foreign_keys,
                }
            })
    }
}

fn migration_value_new(field: &ScalarFieldWalker<'_>) -> Option<sql_schema_describer::DefaultValue> {
    let value = match &field.default_value()? {
        datamodel::DefaultValue::Single(s) => match field.field_type() {
            TypeWalker::Enum(inum) => {
                let corresponding_value = inum
                    .r#enum
                    .values()
                    .find(|val| val.name.as_str() == s.to_string())
                    .expect("could not find enum value");

                PrismaValue::Enum(corresponding_value.final_database_name().to_owned())
            }
            _ => s.clone(),
        },
        datamodel::DefaultValue::Expression(expression) if expression.name == "now" && expression.args.is_empty() => {
            return Some(sql_schema_describer::DefaultValue::NOW)
        }
        datamodel::DefaultValue::Expression(expression)
            if expression.name == "dbgenerated" && expression.args.is_empty() =>
        {
            return Some(sql_schema_describer::DefaultValue::DBGENERATED(String::new()))
        }
        datamodel::DefaultValue::Expression(expression)
            if expression.name == "autoincrement" && expression.args.is_empty() =>
        {
            return Some(sql_schema_describer::DefaultValue::SEQUENCE(String::new()))
        }
        datamodel::DefaultValue::Expression(_) => return None,
    };

    Some(sql_schema_describer::DefaultValue::VALUE(value))
}

fn enum_column_type(field: &ScalarFieldWalker<'_>, database_info: &DatabaseInfo, db_name: &str) -> sql::ColumnType {
    let arity = column_arity(field.arity());
    match database_info.sql_family() {
        SqlFamily::Postgres => sql::ColumnType::pure(sql::ColumnTypeFamily::Enum(db_name.to_owned()), arity),
        SqlFamily::Mysql => sql::ColumnType::pure(
            sql::ColumnTypeFamily::Enum(format!("{}_{}", field.model().db_name(), field.db_name())),
            arity,
        ),
        _ => column_type(field),
    }
}

fn column_type(field: &ScalarFieldWalker<'_>) -> sql::ColumnType {
    column_type_for_scalar_type(&scalar_type_for_field(field), column_arity(field.arity()))
}

fn scalar_type_for_field(field: &ScalarFieldWalker<'_>) -> ScalarType {
    match field.field_type() {
        TypeWalker::Base(ref scalar) => *scalar,
        TypeWalker::Enum(_) => ScalarType::String,
        x => panic!(format!(
            "This field type is not suported here. Field type is {:?} on field {}",
            x,
            field.name()
        )),
    }
}

fn column_arity(arity: FieldArity) -> sql::ColumnArity {
    match &arity {
        FieldArity::Required => sql::ColumnArity::Required,
        FieldArity::List => sql::ColumnArity::List,
        FieldArity::Optional => sql::ColumnArity::Nullable,
    }
}

fn column_type_for_scalar_type(scalar_type: &ScalarType, column_arity: ColumnArity) -> sql::ColumnType {
    match scalar_type {
        ScalarType::Int => sql::ColumnType::pure(sql::ColumnTypeFamily::Int, column_arity),
        ScalarType::Float => sql::ColumnType::pure(sql::ColumnTypeFamily::Float, column_arity),
        ScalarType::Boolean => sql::ColumnType::pure(sql::ColumnTypeFamily::Boolean, column_arity),
        ScalarType::String => sql::ColumnType::pure(sql::ColumnTypeFamily::String, column_arity),
        ScalarType::DateTime => sql::ColumnType::pure(sql::ColumnTypeFamily::DateTime, column_arity),
        ScalarType::Json => sql::ColumnType::pure(sql::ColumnTypeFamily::Json, column_arity),
    }
}

fn add_one_to_one_relation_unique_index(table: &mut sql::Table, column_names: &[String]) {
    // Don't add a duplicate index.
    if table
        .indices
        .iter()
        .any(|index| index.columns == column_names && index.tpe.is_unique())
    {
        return;
    }

    let columns_suffix = column_names.join("_");
    let index = sql::Index {
        name: format!("{}_{}", table.name, columns_suffix),
        columns: column_names.to_owned(),
        tpe: sql::IndexType::Unique,
    };

    table.indices.push(index);
}
