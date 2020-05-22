extern crate datamodel;
use pretty_assertions::assert_eq;

#[test]
fn back_relation_fields_must_be_added() {
    let input = r#"model Blog {
  id    Int     @id
  posts Post[]
}

model Post {
  id Int   @id
}

model Post2 {
  id     Int  @id
  blogId Int
  Blog   Blog @relation(fields: [blogId], references: [id])
}
"#;

    let expected = r#"model Blog {
  id    Int     @id
  posts Post[]
  Post2 Post2[]
}

model Post {
  id     Int   @id
  Blog   Blog? @relation(fields: [blogId], references: [id])
  blogId Int?
}

model Post2 {
  id     Int  @id
  blogId Int
  Blog   Blog @relation(fields: [blogId], references: [id])
}
"#;

    assert_reformat(input, expected);
}

#[test]
#[ignore]
fn must_add_relation_directive_to_an_existing_field() {
    let input = r#"
    model Blog {
      id    Int     @id
      posts Post[]
    }
    
    model Post {
      id     Int   @id
      Blog   Blog? @relation(fields: [blogId])
      blogId Int?
    }    
    "#;

    let expected = r#"model Blog {
  id    Int    @id
  posts Post[]
}

model Post {
  id     Int   @id
  Blog   Blog? @relation(fields: [blogId], references: [id])
  blogId Int?
}
"#;
    assert_reformat(input, expected);
}

fn assert_reformat(schema: &str, expected_result: &str) {
    println!("schema: {:?}", schema);
    let result = datamodel::ast::reformat::Reformatter::new(&schema).reformat_to_string();
    println!("result: {}", result);
    assert_eq!(result, expected_result);
}
