mod ast;
mod enum_renderer;

mod field_renderer;
mod object_renderer;
mod schema_renderer;
mod type_renderer;

use core::schema::*;
use enum_renderer::*;
use field_renderer::*;
use object_renderer::*;
use schema_renderer::*;

use std::{cell::RefCell, collections::HashMap, sync::Weak};
use type_renderer::*;

pub use ast::*;

pub struct DMMFQuerySchemaRenderer;

impl QuerySchemaRenderer<DMMFSchema> for DMMFQuerySchemaRenderer {
  fn render(query_schema: &QuerySchema) -> DMMFSchema {
    let ctx = RenderContext::new();
    let (_, ctx) = query_schema.into_renderer().render(ctx);

    ctx.finalize()
  }
}

pub struct RenderContext {
  /// Aggregator
  schema: RefCell<DMMFSchema>,

  /// Prevents double rendering of elements that are referenced multiple times.
  /// Names of input / output types / enums / models are globally unique.
  rendered: RefCell<HashMap<String, ()>>,
}

impl RenderContext {
  pub fn new() -> Self {
    RenderContext {
      schema: RefCell::new(DMMFSchema::new()),
      rendered: RefCell::new(HashMap::new()),
    }
  }

  pub fn finalize(self) -> DMMFSchema {
    let mut schema = self.schema.into_inner();

    schema.root_query_type = "Query".into();
    schema.root_mutation_type = "Mutation".into();

    schema
  }

  pub fn already_rendered(&self, cache_key: &str) -> bool {
    self.rendered.borrow().contains_key(cache_key)
  }

  pub fn mark_as_rendered(&self, cache_key: String) {
    self.rendered.borrow_mut().insert(cache_key, ());
  }

  pub fn add_enum(&self, name: String, dmmf_enum: DMMFEnum) {
    self.schema.borrow_mut().enums.push(dmmf_enum);
    self.mark_as_rendered(name);
  }

  pub fn add_input_type(&self, input_type: DMMFInputType) {
    self.mark_as_rendered(input_type.name.clone());
    self.schema.borrow_mut().input_types.push(input_type);
  }

  pub fn add_output_type(&self, output_type: DMMFOutputType) {
    self.mark_as_rendered(output_type.name.clone());
    self.schema.borrow_mut().output_types.push(output_type);
  }
}

pub trait Renderer<'a, T> {
  fn render(&self, ctx: RenderContext) -> (T, RenderContext);
}

trait IntoRenderer<'a, T> {
  fn into_renderer(&'a self) -> Box<dyn Renderer<'a, T> + 'a>;
}

impl<'a> IntoRenderer<'a, ()> for QuerySchema {
  fn into_renderer(&'a self) -> Box<Renderer<'a, ()> + 'a> {
    Box::new(DMMFSchemaRenderer::new(self))
  }
}

impl<'a> IntoRenderer<'a, DMMFTypeInfo> for OutputType {
  fn into_renderer(&'a self) -> Box<Renderer<'a, DMMFTypeInfo> + 'a> {
    Box::new(DMMFTypeRenderer::Output(self))
  }
}

impl<'a> IntoRenderer<'a, DMMFTypeInfo> for InputType {
  fn into_renderer(&'a self) -> Box<Renderer<'a, DMMFTypeInfo> + 'a> {
    Box::new(DMMFTypeRenderer::Input(self))
  }
}

impl<'a> IntoRenderer<'a, ()> for EnumType {
  fn into_renderer(&'a self) -> Box<Renderer<'a, ()> + 'a> {
    Box::new(DMMFEnumRenderer::new(self))
  }
}

impl<'a> IntoRenderer<'a, DMMFFieldWrapper> for InputField {
  fn into_renderer(&'a self) -> Box<Renderer<'a, DMMFFieldWrapper> + 'a> {
    Box::new(DMMFFieldRenderer::Input(self))
  }
}

impl<'a> IntoRenderer<'a, DMMFFieldWrapper> for Field {
  fn into_renderer(&'a self) -> Box<Renderer<'a, DMMFFieldWrapper> + 'a> {
    Box::new(DMMFFieldRenderer::Output(self))
  }
}

impl<'a> IntoRenderer<'a, ()> for InputObjectTypeRef {
  fn into_renderer(&'a self) -> Box<Renderer<'a, ()> + 'a> {
    Box::new(DMMFObjectRenderer::Input(Weak::clone(self)))
  }
}

impl<'a> IntoRenderer<'a, ()> for ObjectTypeRef {
  fn into_renderer(&'a self) -> Box<Renderer<'a, ()> + 'a> {
    Box::new(DMMFObjectRenderer::Output(Weak::clone(self)))
  }
}
