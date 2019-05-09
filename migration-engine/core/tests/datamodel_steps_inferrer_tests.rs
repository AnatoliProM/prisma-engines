#![allow(non_snake_case)]

use datamodel::dml::*;
use datamodel::Validator;
use migration_connector::steps::*;
use migration_core::migration::datamodel_migration_steps_inferrer::{
    DataModelMigrationStepsInferrer, DataModelMigrationStepsInferrerImpl,
};

#[test]
fn infer_CreateModel_if_it_does_not_exit_yet() {
    let dm1 = Schema::empty();
    let dm2 = parse(
        r#"
        model Test {
            id: String
        }
    "#,
    );

    let steps = infer(dm1, dm2);
    let expected = vec![
        MigrationStep::CreateModel(CreateModel {
            name: "Test".to_string(),
            db_name: None,
            embedded: false,
        }),
        MigrationStep::CreateField(CreateField {
            model: "Test".to_string(),
            name: "id".to_string(),
            tpe: FieldType::Base(ScalarType::String),
            arity: FieldArity::Required,
            db_name: None,
            is_created_at: None,
            is_updated_at: None,
            id: None,
            default: None,
            scalar_list: None,
        }),
    ];
    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteModel() {
    let dm1 = parse(
        r#"
        model Test {
            id: String
        }
    "#,
    );
    let dm2 = Schema::empty();

    let steps = infer(dm1, dm2);
    let expected = vec![MigrationStep::DeleteModel(DeleteModel {
        name: "Test".to_string(),
    })];
    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateField_if_it_does_not_exist_yet() {
    let dm1 = parse(
        r#"
        model Test {
            id: String
        }
    "#,
    );
    let dm2 = parse(
        r#"
        model Test {
            id: String
            field: Int?
        }
    "#,
    );

    let steps = infer(dm1, dm2);
    let expected = vec![MigrationStep::CreateField(CreateField {
        model: "Test".to_string(),
        name: "field".to_string(),
        tpe: FieldType::Base(ScalarType::Int),
        arity: FieldArity::Optional,
        db_name: None,
        is_created_at: None,
        is_updated_at: None,
        id: None,
        default: None,
        scalar_list: None,
    })];
    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateField_if_relation_field_does_not_exist_yet() {
    let dm1 = parse(
        r#"
        model Blog {
            id: String
        }
        model Post {
            id: String
        }
    "#,
    );
    let dm2 = parse(
        r#"
        model Blog {
            id: String
            posts: Post[]
        }
        model Post {
            id: String
            blog: Blog?
        }
    "#,
    );

    let steps = infer(dm1, dm2);
    let expected = vec![
        MigrationStep::CreateField(CreateField {
            model: "Blog".to_string(),
            name: "posts".to_string(),
            tpe: FieldType::Relation {
                to: "Post".to_string(),
                to_field: "".to_string(),
                name: None,
                on_delete: OnDeleteStrategy::None,
            },
            arity: FieldArity::List,
            db_name: None,
            is_created_at: None,
            is_updated_at: None,
            id: None,
            default: None,
            scalar_list: None,
        }),
        MigrationStep::CreateField(CreateField {
            model: "Post".to_string(),
            name: "blog".to_string(),
            tpe: FieldType::Relation {
                to: "Blog".to_string(),
                to_field: "".to_string(),
                name: None,
                on_delete: OnDeleteStrategy::None,
            },
            arity: FieldArity::Optional,
            db_name: None,
            is_created_at: None,
            is_updated_at: None,
            id: None,
            default: None,
            scalar_list: None,
        }),
    ];
    assert_eq!(steps, expected);
}

// TODO: we will need this in a lot of test files. Extract it.
fn parse(datamodel_string: &'static str) -> Schema {
    let ast = datamodel::parser::parse(&datamodel_string.to_string());
    // TODO: this would need capabilities
    let validator = Validator::new();
    validator.validate(&ast)
}

fn infer(dm1: Schema, dm2: Schema) -> Vec<MigrationStep> {
    DataModelMigrationStepsInferrerImpl::infer(dm1, dm2)
}
