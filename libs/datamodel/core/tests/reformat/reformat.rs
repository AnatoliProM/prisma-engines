extern crate datamodel;
use pretty_assertions::assert_eq;
use std::str;

#[test]
fn test_reformat_model_simple() {
    let input = r#"
        model User { 
            id               Int                   @id 
        }
    "#;

    let expected = r#"model User {
  id Int @id
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn test_reformat_model_complex() {
    let input = r#"
        model User { 
            id Int @id
            fieldA String    @unique // comment on the side
            // comment before
            anotherWeirdFieldName Int 
        }
    "#;

    let expected = r#"model User {
  id                    Int    @id
  fieldA                String @unique // comment on the side
  // comment before
  anotherWeirdFieldName Int
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn comments_in_a_model_must_not_move() {
    let input = r#"
        model User {
          id     Int    @id
          // Comment
          email  String @unique
          // Comment 2
        }
    "#;

    let expected = r#"model User {
  id    Int    @id
  // Comment
  email String @unique
  // Comment 2
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn commented_models_dont_get_removed() {
    let input = r#"
        // model One {
        //   id Int @id
        // }
        
        model Two {
          id Int @id
        }
        
        // model Three {
        //   id Int @id
        // }
    "#;

    let expected = r#"// model One {
//   id Int @id
// }

model Two {
  id Int @id
}

// model Three {
//   id Int @id
// }
"#;

    assert_reformat(input, expected);
}

#[test]
fn test_reformat_config() {
    let input = r#"
        datasource pg { 
            provider = "postgresql"
            url = "postgresql://"
        }
    "#;

    let expected = r#"datasource pg {
  provider = "postgresql"
  url      = "postgresql://"
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn test_reformat_tabs() {
    let input = r#"
        datasource pg {
            provider\t=\t"postgresql"
            url = "postgresql://"
        }
    "#;

    let expected = r#"datasource pg {
  provider = "postgresql"
  url      = "postgresql://"
}
"#;

    assert_reformat(&input.replace("\\t", "\t"), expected);
}

#[test]
fn test_floating_doc_comment() {
    let input = r#"
model a {
  one Int
  two Int
  // bs  b[] @relation(references: [a])
  @@id([one, two])
}

/// ajlsdkfkjasflk
// model ok {}"#;

    // TODO: that the inner comment is moved to the top is not ideal
    let expected = r#"model a {
  one Int
  two Int
  // bs  b[] @relation(references: [a])
  @@id([one, two])
}

/// ajlsdkfkjasflk
// model ok {}
"#;

    assert_reformat(input, expected);
}

#[test]
fn test_floating_doc_comments() {
    let input = r#"
model a {
  one Int
  two Int
  // bs  b[] @relation(references: [a])
  @@id([one, two])
}

// ajlsdkfkjasflk
// ajlsdkfkjasflk
"#;

    let expected = r#"model a {
  one Int
  two Int
  // bs  b[] @relation(references: [a])
  @@id([one, two])
}

// ajlsdkfkjasflk
// ajlsdkfkjasflk
"#;

    assert_reformat(input, expected);
}

#[test]
fn reformatting_enums_must_work() {
    let input = r#"enum Colors {
  RED @map("rett")
  BLUE
  GREEN

  // comment
  ORANGE_AND_KIND_OF_RED @map("super_color")
  
  @@map("the_colors")
}
"#;
    let expected = r#"enum Colors {
  RED    @map("rett")
  BLUE
  GREEN

  // comment
  ORANGE_AND_KIND_OF_RED  @map("super_color")

  @@map("the_colors")
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn reformatting_must_work_when_env_var_is_missing() {
    let input = r#"
        datasource pg { 
            provider = "postgresql"
            url = env("DATABASE_URL")
        }
    "#;

    let expected = r#"datasource pg {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn invalid_lines_must_not_break_reformatting() {
    let input = r#"$ /a/b/c:.
model Post {
  id Int @id
}
"#;

    assert_reformat(input, input);
}

#[test]
fn new_lines_before_first_block_must_be_removed() {
    let input = r#"

model Post {
  id Int @id
}"#;

    let expected = r#"model Post {
  id Int @id
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn new_lines_between_blocks_must_be_reduced_to_one_simple() {
    let input = r#"model Post {
  id Int @id
}


model Blog {
  id Int @id
}
"#;

    let expected = r#"model Post {
  id Int @id
}

model Blog {
  id Int @id
}
"#;

    assert_reformat(input, expected);
}

// TODO: this will only work if we bring back `///` comments. The idea is that `//` are generally not moved. `///` would get attributed to the next block. Hence this test would work again.
#[test]
#[ignore]
fn new_lines_between_blocks_must_be_reduced_to_one_complex() {
    let input = r#"model Post {
  id Int @id
}


// model comment

model Blog {
  id Int @id
}


// source comment

datasource mydb {
  provider = "sqlite"
  url      = "file:dev.db"
}


// enum comment

enum Status {
  ACTIVE
  DONE
}


// type alias comment

type MyString = String          @default("FooBar")


// generator comment

generator js {
    provider = "js"
}


// another model comment

model Comment {
  id Int @id
}
"#;

    let expected = r#"model Post {
  id Int @id
}

// model comment
model Blog {
  id Int @id
}

// source comment
datasource mydb {
  provider = "sqlite"
  url      = "file:dev.db"
}

// enum comment
enum Status {
  ACTIVE
  DONE
}

// type alias comment
type MyString = String @default("FooBar")

// generator comment
generator js {
  provider = "js"
}

// another model comment
model Comment {
  id Int @id
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn model_level_directives_reset_the_table_layout() {
    let input = r#"model Post {
  id Int @id
  aVeryLongName  String
  @@index([a])
  alsoAVeryLongName String
}
"#;

    let expected = r#"model Post {
  id            Int    @id
  aVeryLongName String
  @@index([a])
  alsoAVeryLongName String
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
