extern crate datamodel;

const DATAMODEL_STRING: &str = r#"
model User {
    id Int @id
    createdAt DateTime
    email String @unique
    name String?
    posts Post[] @relation(onDelete: CASCADE)
    profile Profile?
    @@db("user")
}

model Profile {
    id Int @id
    user User
    bio String
    @@db("profile")
}

model Post {
    id Int @id
    createdAt DateTime
    updatedAt DateTime
    title String @default("Default-Title")
    wasLiked Boolean @default(false)
    author User @relation("author")
    published Boolean @default(false)
    categories PostToCategory[]
    @@db("post")
}

model Category {
    id Int @id
    name String
    posts PostToCategory[]
    cat CategoryEnum
    @@db("category")
}

model PostToCategory {
    id Int @id
    post Post
    category Category
    @@db("post_to_category")
}

model A {
    id Int @id
    b B
}

model B {
    id Int @id
    a A
}

enum CategoryEnum {
    A
    B
    C
}
"#;

#[test]
fn test_dmmf_roundtrip() {
    let dml = datamodel::parse(&DATAMODEL_STRING).unwrap();
    let dmmf = datamodel::dmmf::render_to_dmmf(&dml);
    let dml2 = datamodel::dmmf::parse_from_dmmf(&dmmf);
    let rendered = datamodel::render(&dml2).unwrap();

    println!("{}", rendered);

    assert_eq!(DATAMODEL_STRING, rendered);
}
