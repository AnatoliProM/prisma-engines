use super::*;
use datamodel_connector::ScalarFieldType;

/// Datamodel field arity.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum FieldArity {
    Required,
    Optional,
    List,
}

impl FieldArity {
    pub fn is_singular(&self) -> bool {
        self == &FieldArity::Required || self == &FieldArity::Optional
    }
}

/// Datamodel field type.
#[derive(Debug, PartialEq, Clone)]
pub enum FieldType {
    /// This is an enum field, with an enum of the given name.
    Enum(String),
    /// This is a relation field.
    Relation(RelationInfo),
    /// Connector specific field type.
    ConnectorSpecific(ScalarFieldType),
    /// Base (built-in scalar) type.
    Base(ScalarType),
}

impl FieldType {
    pub fn is_relation(&self) -> bool {
        match self {
            Self::Relation(_) => true,
            _ => false,
        }
    }
}

/// Holds information about an id, or priamry key.
#[derive(Debug, PartialEq, Clone)]
pub struct IdInfo {
    /// The strategy which is used to generate the id field.
    pub strategy: IdStrategy,
    /// A sequence used to generate the id.
    pub sequence: Option<Sequence>,
}

/// Describes a singular field on a data source.
/// This doesn't necessarily map 1:1 to fields in the datamodel, as some
/// datamodel fields, notably relation fields, can be backed by multiple
/// data source fields.
#[derive(Debug, PartialEq, Clone)]
pub struct DataSourceField {
    /// Name of the backing DB field (e.g. column name or document key).
    pub name: Option<String>, // todo why is this an option?

    /// Default value of the backing field, if any.
    pub default_value: Option<DefaultValue>,
}

/// Represents a field in a model.
#[derive(Debug, PartialEq, Clone)]
pub struct Field {
    /// Name of the field.
    pub name: String,

    /// The field's arity.
    pub arity: FieldArity,

    /// The field's type.
    pub field_type: FieldType,

    // -------- todo this is duplicated from DataSourceField --------
    /// The database internal name.
    pub database_name: Option<String>,

    /// The default value.
    pub default_value: Option<DefaultValue>,
    // -------- -------------------------------------------- --------
    /// Indicates if the field is unique.
    pub is_unique: bool,

    /// If set, signals that this field is an id field, or
    /// primary key.
    pub id_info: Option<IdInfo>,

    /// Comments associated with this field.
    pub documentation: Option<String>,

    /// If set, signals that this field was internally generated
    /// and should never be displayed to the user.
    pub is_generated: bool,

    /// If set, signals that this field is updated_at and will be updated to now()
    /// automatically.
    pub is_updated_at: bool,

    /// The data source field specifics, like backing fields and defaults.
    pub data_source_fields: Vec<DataSourceField>,
}

impl WithName for Field {
    fn name(&self) -> &String {
        &self.name
    }
    fn set_name(&mut self, name: &str) {
        self.name = String::from(name)
    }
}

impl WithDatabaseName for Field {
    fn database_name(&self) -> &Option<String> {
        &self.database_name
    }

    fn set_database_name(&mut self, database_name: &Option<String>) {
        self.database_name = database_name.clone()
    }
}

impl Field {
    /// Creates a new field with the given name and type.
    pub fn new(name: &str, field_type: FieldType) -> Field {
        Field {
            name: String::from(name),
            arity: FieldArity::Required,
            field_type,
            database_name: None,
            default_value: None,
            is_unique: false,
            id_info: None,
            documentation: None,
            is_generated: false,
            is_updated_at: false,
            data_source_fields: vec![],
        }
    }
    /// Creates a new field with the given name and type, marked as generated and optional.
    pub fn new_generated(name: &str, field_type: FieldType) -> Field {
        Field {
            name: String::from(name),
            arity: FieldArity::Optional,
            field_type,
            database_name: None,
            default_value: None,
            is_unique: false,
            id_info: None,
            documentation: None,
            is_generated: true,
            is_updated_at: false,
            data_source_fields: vec![],
        }
    }
}
