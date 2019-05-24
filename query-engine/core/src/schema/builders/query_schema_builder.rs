use super::*;
use prisma_models::{InternalDataModelRef, ModelRef};
use std::sync::Arc;

/// Query schema builder. Root for query schema building.
pub struct QuerySchemaBuilder<'a> {
  internal_data_model: InternalDataModelRef,
  capabilities: &'a SupportedCapabilities,
  object_type_builder: ObjectTypeBuilder<'a>,
  argument_builder: ArgumentBuilder,
  pub filter_object_type_builder: Arc<FilterObjectTypeBuilder<'a>>,
}

// WIP: The implementation uses Arcs liberally, which might cause memory leaks - this is not an issue at the moment as the schema
// is supposed to live as long as the program lives _for now_.
impl<'a> QuerySchemaBuilder<'a> {
  pub fn new(internal_data_model: &InternalDataModelRef, capabilities: &'a SupportedCapabilities) -> Self {
    let input_type_builder = Arc::new(InputTypeBuilder::new(Arc::clone(internal_data_model)));
    let filter_object_type_builder = Arc::new(FilterObjectTypeBuilder::new(
      Arc::clone(&input_type_builder),
      capabilities,
    ));
    let object_type_builder = ObjectTypeBuilder::new(
      Arc::clone(internal_data_model),
      true,
      capabilities,
      Arc::clone(&filter_object_type_builder),
    );

    let argument_builder = ArgumentBuilder::new(Arc::clone(internal_data_model), Arc::clone(&input_type_builder));

    QuerySchemaBuilder {
      internal_data_model: Arc::clone(internal_data_model),
      capabilities,
      object_type_builder,
      argument_builder,
      filter_object_type_builder,
    }
  }

  pub fn build(&self) -> QuerySchema {
    QuerySchema {
      query: self.build_query_type(),
      mutation: self.build_mutation_type(),
    }
  }

  /// Builds the root query type.
  fn build_query_type(&self) -> OutputType {
    let non_embedded_models: Vec<ModelRef> = self
      .internal_data_model
      .models()
      .iter()
      .filter(|m| !m.is_embedded)
      .map(|m| Arc::clone(m))
      .collect();

    let fields = non_embedded_models
      .into_iter()
      .filter_map(|m| {
        let mut vec = vec![self.all_items_field(Arc::clone(&m))];

        self
          .single_item_field(Arc::clone(&m))
          .into_iter()
          .for_each(|f| vec.push(f));

        Some(vec)
      })
      .flatten()
      .collect();

    OutputType::Object(Arc::new(object_type("Query", fields)))
  }

  /// Builds the root mutation type.
  fn build_mutation_type(&self) -> OutputType {
    OutputType::Object(Arc::new(object_type("Mutation", vec![])))
  }

  /// Builds a "multiple" query arity items field (e.g. "users", "posts", ...) for given model.
  fn all_items_field(&self, model: ModelRef) -> Field {
    let args = self.object_type_builder.many_records_arguments(&model);

    field(
      camel_case(pluralize(model.name.clone())),
      args,
      OutputType::list(OutputType::opt(OutputType::object(
        self.object_type_builder.map_model_object_type(&model),
      ))),
    )
  }

  /// Builds a "single" query arity item field (e.g. "user", "post" ...) for given model.
  fn single_item_field(&self, model: ModelRef) -> Option<Field> {
    self
      .argument_builder
      .where_unique_argument(Arc::clone(&model))
      .map(|arg| {
        field(
          camel_case(model.name.clone()),
          vec![arg],
          OutputType::opt(OutputType::object(
            self.object_type_builder.map_model_object_type(&model),
          )),
        )
      })
  }
}
