pub mod parser;

#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }
    pub fn empty() -> Span {
        Span { start: 0, end: 0 }
    }
    pub fn from_pest(s: &pest::Span) -> Span {
        Span {
            start: s.start(),
            end: s.end(),
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[{} - {}]", self.start, self.end)
    }
}

#[derive(Debug)]
pub enum FieldArity {
    Required,
    Optional,
    List,
}

#[derive(Debug)]
pub struct Comment {
    pub text: String,
    pub is_error: bool,
}

#[derive(Debug)]
pub struct Argument {
    pub name: String,
    pub value: Value,
    pub span: Span,
}

// TODO: Rename to expression.
#[derive(Debug, Clone)]
pub enum Value {
    NumericValue(String, Span),
    BooleanValue(String, Span),
    StringValue(String, Span),
    ConstantValue(String, Span),
    Function(String, Vec<Value>, Span),
}

pub fn describe_value_type(val: &Value) -> &'static str {
    match val {
        Value::NumericValue(_, _) => "Numeric",
        Value::BooleanValue(_, _) => "Boolean",
        Value::StringValue(_, _) => "String",
        Value::ConstantValue(_, _) => "Literal",
        Value::Function(_, _, _) => "Functional",
    }
}

#[derive(Debug)]
pub struct Directive {
    pub name: String,
    pub arguments: Vec<Argument>,
    pub span: Span,
}

pub trait WithDirectives {
    fn directives(&self) -> &Vec<Directive>;
}

pub trait WithComments {
    fn comments(&self) -> &Vec<Comment>;
}

#[derive(Debug)]
pub struct Field {
    pub field_type: String,
    pub field_type_span: Span,
    pub field_link: Option<String>,
    pub name: String,
    pub arity: FieldArity,
    pub default_value: Option<Value>,
    pub directives: Vec<Directive>,
    pub comments: Vec<Comment>,
    pub span: Span,
}

impl WithDirectives for Field {
    fn directives(&self) -> &Vec<Directive> {
        &self.directives
    }
}

impl WithComments for Field {
    fn comments(&self) -> &Vec<Comment> {
        &self.comments
    }
}

#[derive(Debug)]
pub struct Enum {
    pub name: String,
    pub values: Vec<String>,
    pub directives: Vec<Directive>,
    pub comments: Vec<Comment>,
}

impl WithDirectives for Enum {
    fn directives(&self) -> &Vec<Directive> {
        &self.directives
    }
}

impl WithComments for Enum {
    fn comments(&self) -> &Vec<Comment> {
        &self.comments
    }
}

#[derive(Debug)]
pub struct Model {
    pub name: String,
    pub fields: Vec<Field>,
    pub directives: Vec<Directive>,
    pub comments: Vec<Comment>,
}

impl WithDirectives for Model {
    fn directives(&self) -> &Vec<Directive> {
        &self.directives
    }
}

impl WithComments for Model {
    fn comments(&self) -> &Vec<Comment> {
        &self.comments
    }
}

#[derive(Debug)]
pub struct SourceConfig {
    pub name: String,
    // Top level config
    pub properties: Vec<Argument>,
    // Inner properties block
    pub detail_configuration: Vec<Argument>,
    pub comments: Vec<Comment>,
    pub span: Span,
}

impl WithComments for SourceConfig {
    fn comments(&self) -> &Vec<Comment> {
        &self.comments
    }
}

#[derive(Debug)]
pub enum Top {
    Enum(Enum),
    Model(Model),
    Source(SourceConfig),
}

#[derive(Debug)]
pub struct Schema {
    pub models: Vec<Top>,
    pub comments: Vec<Comment>,
}
