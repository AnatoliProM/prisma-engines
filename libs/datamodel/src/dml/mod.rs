// TODOs to answer together with rust teams:
// * Should this structure be mutatble or immutable?
// * Should this structure contain circular references? (Would make renaming models/fields MUCH easier)
// * How do we handle ocnnector specific settings, like indeces? Maybe inheritance, traits and having a Connector<T>?

use chrono::{DateTime, Utc};
use std::str::FromStr;
use validator::value::ValueParserError;
use serde::{Serialize, Deserialize};

pub mod validator;

// Setters are a bit untypical for rust,
// but we want to have "composeable" struct creation.
pub trait WithName {
    fn name(&self) -> &String;
    fn set_name(&mut self, name: &String);
}

pub trait WithDatabaseName {
    fn database_name(&self) -> &Option<String>;
    fn set_database_name(&mut self, database_name: &Option<String>);
}

// This is duplicate for now, but explicitely required
// since we want to seperate ast and dml.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FieldArity {
    Required,
    Optional,
    List,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub text: String,
    pub is_error: bool,
}

#[derive(Debug, Copy, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum ScalarType {
    Int,
    Float,
    Decimal,
    Boolean,
    String,
    DateTime,
    Enum,
}

// TODO, Check if data types are correct
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Value {
    Int(i32),
    Float(f32),
    Decimal(f32),
    Boolean(bool),
    String(String),
    DateTime(DateTime<Utc>),
    ConstantLiteral(String),
}

// TODO: Maybe we include a seperate struct for relations which can be generic?
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    Enum { enum_type: String },
    Relation { to: String, to_field: String, name: Option<String>, on_delete: OnDeleteStrategy },
    ConnectorSpecific { base_type: ScalarType, connector_type: Option<String> },
    Base(ScalarType)
}



#[derive(Debug, Copy, PartialEq, Clone)]
pub enum IdStrategy {
    Auto,
    None,
}

impl FromStr for IdStrategy {
    type Err = ValueParserError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AUTO" => Ok(IdStrategy::Auto),
            "NONE" => Ok(IdStrategy::None),
            _ => Err(ValueParserError::new(format!("Invalid id strategy {}.", s))),
        }
    }
}

#[derive(Debug, Copy, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum ScalarListStrategy {
    Embedded,
    Relation,
}

impl FromStr for ScalarListStrategy {
    type Err = ValueParserError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "EMBEDDED" => Ok(ScalarListStrategy::Embedded),
            "RELATION" => Ok(ScalarListStrategy::Relation),
            _ => Err(ValueParserError::new(format!("Invalid scalar list strategy {}.", s))),
        }
    }
}

#[derive(Debug, Copy, PartialEq, Clone, Serialize, Deserialize)]
pub enum OnDeleteStrategy {
    Cascade,
    None
}

impl FromStr for OnDeleteStrategy {
    type Err = ValueParserError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CASCADE" => Ok(OnDeleteStrategy::Cascade),
            "NONE" => Ok(OnDeleteStrategy::None),
            _ => Err(ValueParserError::new(format!("Invalid onDelete strategy {}.", s)))
        }
    }
}


#[derive(Debug, PartialEq, Clone)]
pub struct Sequence {
    pub name: String,
    pub initial_value: i32,
    pub allocation_size: i32,
}

impl WithName for Sequence {
    fn name(&self) -> &String {
        &self.name
    }
    fn set_name(&mut self, name: &String) {
        self.name = name.clone()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Field {
    pub name: String,
    pub arity: FieldArity,
    pub field_type: FieldType,
    pub database_name: Option<String>,
    pub default_value: Option<Value>,
    pub is_unique: bool,
    // TODO: isn't `is_id` implied if the `id_strategy` field is Some()?
    pub is_id: bool,
    pub id_strategy: Option<IdStrategy>,
    // TODO: Not sure if a sequence should be a member of field.
    pub id_sequence: Option<Sequence>,
    pub scalar_list_strategy: Option<ScalarListStrategy>,
    pub comments: Vec<Comment>,
}

impl WithName for Field {
    fn name(&self) -> &String {
        &self.name
    }
    fn set_name(&mut self, name: &String) {
        self.name = name.clone()
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
    fn new(name: &String, field_type: &FieldType) -> Field {
        Field {
            name: name.clone(),
            arity: FieldArity::Required,
            field_type: field_type.clone(),
            database_name: None,
            default_value: None,
            is_unique: false,
            is_id: false,
            id_strategy: None,
            id_sequence: None,
            scalar_list_strategy: None,
            comments: vec![],
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Enum {
    pub name: String,
    pub values: Vec<String>,
    pub comments: Vec<Comment>,
}

impl WithName for Enum {
    fn name(&self) -> &String {
        &self.name
    }
    fn set_name(&mut self, name: &String) {
        self.name = name.clone()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Model {
    pub name: String,
    pub fields: Vec<Field>,
    pub comments: Vec<Comment>,
    pub database_name: Option<String>,
    pub is_embedded: bool,
}

impl Model {
    fn new(name: &String) -> Model {
        Model {
            name: name.clone(),
            fields: vec![],
            comments: vec![],
            database_name: None,
            is_embedded: false,
        }
    }

    pub fn find_field(&self, name: String) -> Option<Field> {
        self.fields.iter().find(|f| f.name == name).map(|f| f.clone())
    }
}

impl WithName for Model {
    fn name(&self) -> &String { &self.name }
    fn set_name(&mut self, name: &String) { self.name = name.clone() }
}

impl WithDatabaseName for Model {
    fn database_name(&self) -> &Option<String> { &self.database_name }
    fn set_database_name(&mut self, database_name: &Option<String>) { self.database_name = database_name.clone() }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ModelOrEnum {
    Enum(Enum),
    Model(Model)
}

#[derive(Debug, PartialEq, Clone)]
pub struct Schema {
    pub models: Vec<ModelOrEnum>,
    pub comments: Vec<Comment>,
}

impl Schema {
    fn new() -> Schema {
        Schema {
            models: vec![],
            comments: vec![],
        }
    }

    pub fn empty() -> Schema {
        Self::new()
    }

    pub fn has_model(&self, name: String) -> bool {
        for model in &self.models {
            match model {
                ModelOrEnum::Model(m) => {
                    if(m.name() == &name) {
                        return true;
                    }
                },
                _ => {},
            }
        }
        false
    }

    pub fn models(&self) -> Vec<Model> {
        let mut result = Vec::new();
        for model in &self.models {
            match model {
                ModelOrEnum::Model(m) => result.push(m.clone()),
                _ => {},
            }
        }
        result
    }

    pub fn find_model(&self, name: String) -> Option<Model> {
        self.models().iter().find(|m| m.name == name).map(|m| m.clone())
    }
}
