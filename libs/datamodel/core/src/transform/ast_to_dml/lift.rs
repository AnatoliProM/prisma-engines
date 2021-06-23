use super::super::attributes::AllAttributes;
use crate::{
    ast,
    common::preview_features::PreviewFeature,
    diagnostics::{DatamodelError, Diagnostics},
    dml,
    transform::{
        ast_to_dml::db::{ParserDatabase, ScalarFieldType},
        helpers::ValueValidator,
    },
    Datasource,
};
use datamodel_connector::connector_error::{ConnectorError, ErrorKind};
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// Helper for lifting a datamodel.
///
/// When lifting, the AST is converted to the real datamodel, and additional
/// semantics are attached.
pub struct LiftAstToDml<'a> {
    attributes: AllAttributes,
    db: &'a ParserDatabase<'a>,
    diagnostics: &'a mut Diagnostics,
}

impl<'a> LiftAstToDml<'a> {
    /// Creates a new instance, with all builtin attributes and
    /// the attributes defined by the given sources registered.
    ///
    /// The attributes defined by the given sources will be namespaced.
    pub(crate) fn new(
        preview_features: &HashSet<PreviewFeature>,
        db: &'a ParserDatabase<'a>,
        diagnostics: &'a mut Diagnostics,
    ) -> LiftAstToDml<'a> {
        LiftAstToDml {
            attributes: AllAttributes::new(preview_features),
            db,
            diagnostics,
        }
    }

    pub fn lift(&mut self) -> dml::Datamodel {
        let mut schema = dml::Datamodel::new();

        for (top_id, ast_obj) in self.db.ast().iter_tops() {
            match ast_obj {
                ast::Top::Enum(en) => schema.add_enum(self.lift_enum(en)),
                ast::Top::Model(ty) => schema.add_model(self.lift_model(top_id, ty)),
                ast::Top::Source(_) => { /* Source blocks are explicitly ignored by the validator */ }
                ast::Top::Generator(_) => { /* Generator blocks are explicitly ignored by the validator */ }
                ast::Top::Type(_) => { /* Type blocks are inlined */ }
            }
        }

        schema
    }

    /// Internal: Validates a model AST node and lifts it to a DML model.
    fn lift_model(&mut self, model_id: ast::TopId, ast_model: &ast::Model) -> dml::Model {
        let mut model = dml::Model::new(ast_model.name.name.clone(), None);
        model.documentation = ast_model.documentation.clone().map(|comment| comment.text);

        let active_connector = self.db.active_connector();

        // We iterate over scalar fields, then relation fields, but we want the
        // order of fields in the dml::Model to match the order of the fields in
        // the AST, so we need this bit of extra bookkeeping.
        let mut field_ids_for_sorting: HashMap<&str, ast::FieldId> = HashMap::with_capacity(ast_model.fields.len());

        for (field_id, scalar_field_type) in self.db.iter_model_scalar_fields(model_id) {
            let ast_field = &ast_model[field_id];
            let arity = self.lift_field_arity(&ast_field.arity);
            let mut attributes = Vec::with_capacity(ast_field.attributes.len());

            let field_type = self.lift_scalar_field_type(ast_field, scalar_field_type, &mut attributes);

            attributes.extend(ast_field.attributes.iter().cloned());

            let mut field = dml::ScalarField::new(&ast_field.name.name, arity, field_type);
            field.documentation = ast_field.documentation.clone().map(|comment| comment.text);
            let mut field = dml::Field::ScalarField(field);

            if let Err(mut errs) = self.attributes.field.validate_and_apply(&attributes, &mut field) {
                self.diagnostics.append(&mut errs);
            }
            field_ids_for_sorting.insert(&ast_field.name.name, field_id);
            model.add_field(field);
        }

        for (field_id, relation_field) in self.db.iter_model_relation_fields(model_id) {
            let ast_field = &ast_model[field_id];
            let arity = self.lift_field_arity(&ast_field.arity);
            let target_model = &self.db.ast()[relation_field.referenced_model].as_model().unwrap();
            let relation_info = dml::RelationInfo::new(&target_model.name.name);

            let mut field = dml::RelationField::new(&ast_field.name.name, arity, arity, relation_info);

            field.supports_restrict_action(
                active_connector.supports_referential_action(dml::ReferentialAction::Restrict),
            );
            field.emulates_referential_actions(active_connector.emulates_referential_actions());

            field.documentation = ast_field.documentation.clone().map(|comment| comment.text);
            let mut field = dml::Field::RelationField(field);

            if let Err(mut errs) = self.attributes.field.validate_and_apply(ast_field, &mut field) {
                self.diagnostics.append(&mut errs);
            }

            field_ids_for_sorting.insert(&ast_field.name.name, field_id);
            model.add_field(field)
        }

        model.fields.sort_by_key(|f| field_ids_for_sorting.get(f.name()));

        if let Err(mut err) = self.attributes.model.validate_and_apply(ast_model, &mut model) {
            self.diagnostics.append(&mut err);
        }

        model
    }

    /// Internal: Validates an enum AST node.
    fn lift_enum(&mut self, ast_enum: &ast::Enum) -> dml::Enum {
        let mut en = dml::Enum::new(&ast_enum.name.name, vec![]);

        let supports_enums = match self.db.datasource() {
            Some(source) => source.active_connector.supports_enums(),
            None => true,
        };

        if !supports_enums {
            self.diagnostics.push_error(DatamodelError::new_validation_error(
                &format!(
                    "You defined the enum `{}`. But the current connector does not support enums.",
                    &ast_enum.name.name
                ),
                ast_enum.span,
            ));
            return en;
        }

        for ast_enum_value in &ast_enum.values {
            match self.lift_enum_value(ast_enum_value) {
                Ok(value) => en.add_value(value),
                Err(mut err) => self.diagnostics.append(&mut err),
            }
        }

        if en.values.is_empty() {
            self.diagnostics.push_error(DatamodelError::new_validation_error(
                "An enum must have at least one value.",
                ast_enum.span,
            ))
        }

        en.documentation = ast_enum.documentation.clone().map(|comment| comment.text);

        if let Err(mut err) = self.attributes.enm.validate_and_apply(ast_enum, &mut en) {
            self.diagnostics.append(&mut err);
        }

        en
    }

    /// Internal: Validates an enum value AST node.
    fn lift_enum_value(&self, ast_enum_value: &ast::EnumValue) -> Result<dml::EnumValue, Diagnostics> {
        let mut enum_value = dml::EnumValue::new(&ast_enum_value.name.name);
        enum_value.documentation = ast_enum_value.documentation.clone().map(|comment| comment.text);

        self.attributes
            .enm_value
            .validate_and_apply(ast_enum_value, &mut enum_value)?;

        Ok(enum_value)
    }

    /// Internal: Lift a field's arity.
    fn lift_field_arity(&self, ast_field: &ast::FieldArity) -> dml::FieldArity {
        match ast_field {
            ast::FieldArity::Required => dml::FieldArity::Required,
            ast::FieldArity::Optional => dml::FieldArity::Optional,
            ast::FieldArity::List => dml::FieldArity::List,
        }
    }

    fn lift_scalar_field_type(
        &mut self,
        ast_field: &ast::Field,
        scalar_field_type: &ScalarFieldType,
        collected_attributes: &mut Vec<ast::Attribute>,
    ) -> dml::FieldType {
        match scalar_field_type {
            ScalarFieldType::Enum(enum_id) => {
                let enum_name = &self.db.ast()[*enum_id].as_enum().unwrap().name.name;
                dml::FieldType::Enum(enum_name.to_owned())
            }
            ScalarFieldType::Unsupported => lift_unsupported_field_type(
                ast_field,
                ast_field.field_type.as_unsupported().unwrap().0,
                self.db.datasource(),
                self.diagnostics,
            ),
            ScalarFieldType::Alias(top_id) => {
                let alias = self.db.ast()[*top_id].as_type_alias().unwrap();
                collected_attributes.extend(alias.attributes.iter().cloned());
                self.lift_scalar_field_type(alias, self.db.alias_scalar_field_type(top_id), collected_attributes)
            }
            ScalarFieldType::BuiltInScalar => {
                let scalar_type: dml::ScalarType = ast_field.field_type.unwrap_supported().name.parse().unwrap();
                let native_type = self
                    .db
                    .datasource()
                    .and_then(|datasource| lift_native_type(ast_field, &scalar_type, datasource, self.diagnostics));
                dml::FieldType::Scalar(scalar_type, None, native_type)
            }
        }
    }
}

fn lift_native_type(
    ast_field: &ast::Field,
    scalar_type: &dml::ScalarType,
    datasource: &Datasource,
    diagnostics: &mut Diagnostics,
) -> Option<dml::NativeTypeInstance> {
    let connector = &datasource.active_connector;
    let prefix = format!("{}{}", datasource.name, ".");

    let type_specifications_with_invalid_datasource_name = ast_field
        .attributes
        .iter()
        .filter(|dir| dir.name.name.contains('.') && !dir.name.name.starts_with(&prefix))
        .collect_vec();

    if !type_specifications_with_invalid_datasource_name.is_empty() {
        let incorrect_type_specification = type_specifications_with_invalid_datasource_name.first().unwrap();
        let mut type_specification_name_split = incorrect_type_specification.name.name.split('.');
        let given_prefix = type_specification_name_split.next().unwrap();
        diagnostics.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::InvalidPrefixForNativeTypes {
                given_prefix: String::from(given_prefix),
                expected_prefix: datasource.name.clone(),
                suggestion: format!("{}{}", prefix, type_specification_name_split.next().unwrap()),
            })
            .to_string(),
            incorrect_type_specification.span,
        ));
        return None;
    }

    let type_specifications = ast_field
        .attributes
        .iter()
        .filter(|dir| dir.name.name.starts_with(&prefix))
        .collect_vec();

    let type_specification = type_specifications.first();

    if type_specifications.len() > 1 {
        diagnostics.push_error(DatamodelError::new_duplicate_attribute_error(
            &prefix,
            type_specification.unwrap().span,
        ));
        return None;
    }

    // convert arguments to string if possible
    let number_args = type_specification.map(|dir| dir.arguments.clone());
    let args = if let Some(number) = number_args {
        number
            .iter()
            .map(|arg| ValueValidator::new(&arg.value).raw())
            .collect_vec()
    } else {
        vec![]
    };

    let x = type_specification.map(|dir| dir.name.name.trim_start_matches(&prefix))?;
    let constructor = if let Some(cons) = connector.find_native_type_constructor(x) {
        cons
    } else {
        diagnostics.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::NativeTypeNameUnknown {
                native_type: x.parse().unwrap(),
                connector_name: datasource.active_provider.clone(),
            })
            .to_string(),
            type_specification.unwrap().span,
        ));
        return None;
    };

    let number_of_args = args.len();

    if number_of_args < constructor._number_of_args
        || ((number_of_args > constructor._number_of_args) && constructor._number_of_optional_args == 0)
    {
        diagnostics.push_error(DatamodelError::new_argument_count_missmatch_error(
            x,
            constructor._number_of_args,
            number_of_args,
            type_specification.unwrap().span,
        ));
        return None;
    }

    if number_of_args > constructor._number_of_args + constructor._number_of_optional_args
        && constructor._number_of_optional_args > 0
    {
        diagnostics.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::OptionalArgumentCountMismatchError {
                native_type: x.parse().unwrap(),
                optional_count: constructor._number_of_optional_args,
                given_count: number_of_args,
            })
            .to_string(),
            type_specification.unwrap().span,
        ));
        return None;
    }

    // check for compatibility with scalar type
    if !constructor.prisma_types.contains(scalar_type) {
        diagnostics.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::IncompatibleNativeType {
                native_type: x.parse().unwrap(),
                field_type: scalar_type.to_string(),
                expected_types: constructor.prisma_types.iter().map(|s| s.to_string()).join(" or "),
            })
            .to_string(),
            type_specification.unwrap().span,
        ));
        return None;
    }

    match connector.parse_native_type(x, args) {
        Err(connector_error) => {
            diagnostics.push_error(DatamodelError::new_connector_error(
                &connector_error.to_string(),
                type_specification.unwrap().span,
            ));
            None
        }
        Ok(parsed_native_type) => Some(parsed_native_type),
    }
}

fn lift_unsupported_field_type(
    ast_field: &ast::Field,
    unsupported_lit: &str,
    source: Option<&Datasource>,
    diagnostics: &mut Diagnostics,
) -> dml::FieldType {
    static TYPE_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"(?x)
    ^                           # beginning of the string
    (?P<prefix>[^(]+)           # a required prefix that is any character until the first opening brace
    (?:\((?P<params>.*?)\))?    # (optional) an opening parenthesis, a closing parenthesis and captured params in-between
    (?P<suffix>.+)?             # (optional) captured suffix after the params until the end of the string
    $                           # end of the string
    "#).unwrap()
    });

    if let Some(source) = source {
        let connector = &source.active_connector;

        if let Some(captures) = TYPE_REGEX.captures(unsupported_lit) {
            let prefix = captures.name("prefix").unwrap().as_str().trim();

            let params = captures.name("params");
            let args = match params {
                None => vec![],
                Some(params) => params.as_str().split(',').map(|s| s.trim().to_string()).collect(),
            };

            if let Ok(native_type) = connector.parse_native_type(prefix, args) {
                let prisma_type = connector.scalar_type_for_native_type(native_type.serialized_native_type.clone());

                let msg = format!(
                        "The type `Unsupported(\"{}\")` you specified in the type definition for the field `{}` is supported as a native type by Prisma. Please use the native type notation `{} @{}.{}` for full support.",
                        unsupported_lit, ast_field.name.name, prisma_type.to_string(), &source.name, native_type.render()
                    );

                diagnostics.push_error(DatamodelError::new_validation_error(&msg, ast_field.span));
            }
        }
    }

    dml::FieldType::Unsupported(unsupported_lit.into())
}
