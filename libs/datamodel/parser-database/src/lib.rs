#![deny(unsafe_code, rust_2018_idioms)]

//! See the docs on [ParserDatabase](./struct.ParserDatabase.html).
//!
//! ## Terminology
//!
//! Names:
//!
//! - _mapped name_: the name inside an `@map()` or `@@map()` attribute of a model, field, enum or
//!   enum value. This is used to determine what the name of the Prisma schema item is in the
//!   database.
//! - _database name_: the name in the database, once both the name of the item and the mapped
//!   name have been taken into account. The logic is always the same: if a mapped name is defined,
//!   then the database name is the mapped name, otherwise it is the name of the item.
//! - _foreign key name_: the name inside the `map: ...` argument inside an `@relation()`
//!   attribute. This is taken as the database name of the constraint backing the relation, on the
//!   connectors where that makes sense.

pub mod value_validator;
pub mod walkers;

mod attributes;
mod context;
mod indexes;
mod names;
mod relations;
mod types;

pub use names::reserved_model_names;
pub use schema_ast::ast;
pub use types::{IndexAlgorithm, IndexType, ScalarFieldType, ScalarType, SortOrder};

use self::{context::Context, relations::Relations, types::Types};
use diagnostics::{DatamodelError, Diagnostics};
use names::Names;
use value_validator::ValueValidator;

/// ParserDatabase is a container for a Schema AST, together with information
/// gathered during schema validation. Each validation step enriches the
/// database with information that can be used to work with the schema, without
/// changing the AST. Instantiating with `ParserDatabase::new()` will perform a
/// number of validations and make sure the schema makes sense, but it cannot
/// fail. In case the schema is invalid, diagnostics will be created and the
/// resolved information will be incomplete.
///
/// Validations are carried out in the following order:
///
/// - The AST is walked a first time to resolve names: to each relevant
///   identifier, we attach an ID that can be used to reference the
///   corresponding item (model, enum, field, ...)
/// - The AST is walked a second time to resolve types. For each field and each
///   type alias, we look at the type identifier and resolve what it refers to.
/// - The AST is walked a third time to validate attributes on models and
///   fields.
/// - Global validations are then performed on the mostly validated schema.
///   Currently only index name collisions.
///
/// ## Lifetimes
///
/// Throughout the ParserDatabase implementation, you will see many lifetime
/// annotations. The only significant lifetime is the lifetime of the reference
/// to the AST contained in ParserDatabase, that we call by convention `'ast`.
/// Apart from that, everything should be owned or locally borrowed, to keep
/// lifetime management simple.
pub struct ParserDatabase<'ast> {
    ast: &'ast ast::SchemaAst,
    names: Names<'ast>,
    types: Types<'ast>,
    relations: Relations<'ast>,
}

impl<'ast> ParserDatabase<'ast> {
    /// See the docs on [ParserDatabase](/struct.ParserDatabase.html).
    pub fn new(ast: &'ast ast::SchemaAst, diagnostics: Diagnostics) -> (Self, Diagnostics) {
        let db = ParserDatabase {
            ast,
            names: Names::default(),
            types: Types::default(),
            relations: Relations::default(),
        };

        let mut ctx = Context::new(db, diagnostics);

        // First pass: resolve names.
        names::resolve_names(&mut ctx);

        // Return early on name resolution errors.
        if ctx.has_errors() {
            return ctx.finish();
        }

        // Second pass: resolve top-level items and field types.
        types::resolve_types(&mut ctx);

        // Return early on type resolution errors.
        if ctx.has_errors() {
            return ctx.finish();
        }

        // Third pass: validate model and field attributes. All these
        // validations should be _order independent_ and only rely on
        // information from previous steps, not from other attributes.
        attributes::resolve_attributes(&mut ctx);

        // Fourth step: relation inference
        relations::infer_relations(&mut ctx);

        // Fifth step: infer implicit indices
        indexes::infer_implicit_indexes(&mut ctx);

        ctx.finish()
    }

    pub fn alias_scalar_field_type(&self, alias_id: &ast::AliasId) -> &ScalarFieldType {
        &self.types.type_aliases[alias_id]
    }

    pub fn ast(&self) -> &'ast ast::SchemaAst {
        self.ast
    }

    pub fn find_model_field(&self, model_id: ast::ModelId, field_name: &str) -> Option<ast::FieldId> {
        self.names.model_fields.get(&(model_id, field_name)).cloned()
    }

    pub fn get_enum_database_name(&self, enum_id: ast::EnumId) -> Option<&'ast str> {
        self.types.enum_attributes[&enum_id].mapped_name
    }

    pub fn get_enum_value_database_name(&self, enum_id: ast::EnumId, value_idx: u32) -> Option<&'ast str> {
        self.types.enum_attributes[&enum_id]
            .mapped_values
            .get(&value_idx)
            .cloned()
    }
}
