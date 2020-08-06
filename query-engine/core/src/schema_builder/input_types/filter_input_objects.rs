use super::*;
use std::sync::Arc;

pub(crate) fn scalar_filter_object_type(ctx: &mut BuilderContext, model: &ModelRef) -> InputObjectTypeWeakRef {
    let object_name = format!("{}ScalarWhereInput", model.name);
    return_cached_input!(ctx, &object_name);

    let input_object = Arc::new(init_input_object_type(object_name.clone()));
    ctx.cache_input_type(object_name, input_object.clone());

    let weak_ref = Arc::downgrade(&input_object);
    let mut input_fields = vec![
        input_field(
            "AND",
            InputType::opt(InputType::list(InputType::object(weak_ref.clone()))),
            None,
        ),
        input_field(
            "OR",
            InputType::opt(InputType::list(InputType::object(weak_ref.clone()))),
            None,
        ),
        input_field(
            "NOT",
            InputType::opt(InputType::list(InputType::object(weak_ref.clone()))),
            None,
        ),
    ];

    let fields: Vec<ScalarFieldRef> = model.fields().scalar();
    let mut fields: Vec<InputField> = fields.iter().flat_map(|f| map_input_field(f)).collect();

    input_fields.append(&mut fields);
    input_object.set_fields(input_fields);

    weak_ref
}

pub(crate) fn where_object_type(ctx: &mut BuilderContext, model: &ModelRef) -> InputObjectTypeWeakRef {
    let name = format!("{}WhereInput", model.name);
    return_cached_input!(ctx, &name);

    let input_object = Arc::new(init_input_object_type(name.clone()));
    ctx.cache_input_type(name, input_object.clone());

    let weak_ref = Arc::downgrade(&input_object);
    let mut fields = vec![
        input_field(
            "AND",
            InputType::opt(InputType::list(InputType::object(weak_ref.clone()))),
            None,
        ),
        input_field(
            "OR",
            InputType::opt(InputType::list(InputType::object(weak_ref.clone()))),
            None,
        ),
        input_field(
            "NOT",
            InputType::opt(InputType::list(InputType::object(weak_ref.clone()))),
            None,
        ),
    ];

    let mut scalar_input_fields: Vec<InputField> = model
        .fields()
        .scalar()
        .iter()
        .map(|sf| map_input_field(sf))
        .flatten()
        .collect();

    let mut relational_input_fields: Vec<InputField> = model
        .fields()
        .relation()
        .iter()
        .map(|rf| input_fields::map_relation_filter_input_field(ctx, rf))
        .flatten()
        .collect();

    fields.append(&mut scalar_input_fields);
    fields.append(&mut relational_input_fields);

    input_object.set_fields(fields);
    weak_ref
}

pub(crate) fn where_unique_object_type(ctx: &mut BuilderContext, model: &ModelRef) -> InputObjectTypeWeakRef {
    let name = format!("{}WhereUniqueInput", model.name);
    return_cached_input!(ctx, &name);

    let mut x = init_input_object_type(name.clone());
    x.is_one_of = true;

    let input_object = Arc::new(x);
    ctx.cache_input_type(name, input_object.clone());

    // Single unique or ID fields.
    let unique_fields: Vec<ScalarFieldRef> = model.fields().scalar().into_iter().filter(|f| f.unique()).collect();

    let mut fields: Vec<InputField> = unique_fields
        .into_iter()
        .map(|sf| {
            let name = sf.name.clone();
            let typ = map_optional_input_type(&sf);

            input_field(name, typ, None)
        })
        .collect();

    // @@unique compound fields.
    let compound_unique_fields: Vec<InputField> = model
        .unique_indexes()
        .into_iter()
        .map(|index| {
            let typ = compound_field_unique_object_type(ctx, index.name.as_ref(), index.fields());
            let name = compound_index_field_name(index);

            input_field(name, InputType::opt(InputType::object(typ)), None)
        })
        .collect();

    // @@id compound field (there can be only one per model).
    let id_fields = model.fields().id();
    let compound_id_field: Option<InputField> = if id_fields.as_ref().map(|f| f.len() > 1).unwrap_or(false) {
        id_fields.map(|fields| {
            let name = compound_id_field_name(&fields.iter().map(|f| f.name.as_ref()).collect::<Vec<&str>>());
            let typ = compound_field_unique_object_type(ctx, None, fields);

            input_field(name, InputType::opt(InputType::object(typ)), None)
        })
    } else {
        None
    };

    fields.extend(compound_unique_fields);
    fields.extend(compound_id_field);

    input_object.set_fields(fields);

    Arc::downgrade(&input_object)
}

/// Generates and caches an input object type for a compound field.
fn compound_field_unique_object_type(
    ctx: &mut BuilderContext,
    alias: Option<&String>,
    from_fields: Vec<ScalarFieldRef>,
) -> InputObjectTypeWeakRef {
    let name = format!("{}CompoundUniqueInput", compound_object_name(alias, &from_fields));
    return_cached_input!(ctx, &name);

    let input_object = Arc::new(init_input_object_type(name.clone()));
    ctx.cache_input_type(name, input_object.clone());

    let object_fields = from_fields
        .into_iter()
        .map(|field| {
            let name = field.name.clone();
            let typ = map_required_input_type(&field);

            input_field(name, typ, None)
        })
        .collect();

    input_object.set_fields(object_fields);
    Arc::downgrade(&input_object)
}

fn map_input_field(field: &ScalarFieldRef) -> Vec<InputField> {
    filter_arguments::get_field_filters(&ModelField::Scalar(field.clone()))
        .into_iter()
        .map(|arg| {
            let field_name = format!("{}{}", field.name, arg.suffix);
            let mapped = map_required_input_type(&field);

            if arg.is_list && field.is_required {
                input_field(field_name, InputType::opt(InputType::list(mapped)), None)
            } else if arg.is_list && !field.is_required {
                input_field(
                    field_name,
                    InputType::opt(InputType::null(InputType::list(mapped))),
                    None,
                )
            } else {
                input_field(field_name, InputType::opt(mapped), None)
            }
        })
        .collect()
}
