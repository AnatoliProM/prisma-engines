use crate::common::*;
use datamodel::{common::ScalarType, dml};

#[test]
fn parse_scalar_types() {
    let dml = r#"
    model User {
        id Int @id
        firstName String
        age Int
        isPro Boolean
        balance Decimal
        averageGrade Float
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_field("firstName")
        .assert_base_type(&ScalarType::String);
    user_model.assert_has_field("age").assert_base_type(&ScalarType::Int);
    user_model
        .assert_has_field("isPro")
        .assert_base_type(&ScalarType::Boolean);
    user_model
        .assert_has_field("balance")
        .assert_base_type(&ScalarType::Decimal);
    user_model
        .assert_has_field("averageGrade")
        .assert_base_type(&ScalarType::Float);
}

#[test]
fn parse_field_arity() {
    let dml = r#"
    model Post {
        id Int @id
        text String
        photo String?
        comments String[]
    }
    "#;

    let schema = parse(dml);
    let post_model = schema.assert_has_model("Post");
    post_model
        .assert_has_field("text")
        .assert_base_type(&ScalarType::String)
        .assert_arity(&dml::FieldArity::Required);
    post_model
        .assert_has_field("photo")
        .assert_base_type(&ScalarType::String)
        .assert_arity(&dml::FieldArity::Optional);
    post_model
        .assert_has_field("comments")
        .assert_base_type(&ScalarType::String)
        .assert_arity(&dml::FieldArity::List);
}
