use migration_engine_tests::multi_engine_test_api::*;

const BASIC_ENUM_DM: &str = r#"
model Cat {
    id Int @id
    mood CatMood
}

enum CatMood {
    HAPPY
    HUNGRY
}
"#;

#[test_connector(capabilities(Enums))]
fn an_enum_can_be_turned_into_a_model(api: TestApi) {
    let engine = api.new_engine();

    engine
        .schema_push(BASIC_ENUM_DM)
        .send_sync()
        .unwrap()
        .assert_green()
        .unwrap();

    let enum_name = if api.lower_cases_table_names() {
        "cat_mood"
    } else if api.is_mysql() {
        "Cat_mood"
    } else {
        "CatMood"
    };

    #[allow(clippy::redundant_closure)]
    engine.assert_schema().assert_enum(enum_name, |enm| Ok(enm)).unwrap();

    let dm2 = r#"
        model Cat {
            id Int @id
            moodId Int
            mood CatMood @relation(fields: [moodId], references: [id])
        }

        model CatMood {
            id Int @id
            description String
            biteRisk Int
            c        Cat[]
        }
    "#;

    engine.schema_push(dm2).send_sync().unwrap().assert_green().unwrap();

    engine
        .assert_schema()
        .assert_table("Cat", |table| {
            table.assert_columns_count(2)?.assert_column("moodId", Ok)
        })
        .unwrap()
        .assert_table("CatMood", |table| table.assert_column_count(3))
        .unwrap()
        .assert_has_no_enum("CatMood")
        .unwrap();
}

#[test_connector(capabilities(Enums))]
fn variants_can_be_added_to_an_existing_enum(api: TestApi) {
    let engine = api.new_engine();
    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HUNGRY
        }
    "#;

    engine.schema_push(dm1).send_sync().unwrap().assert_green().unwrap();

    let enum_name = if api.lower_cases_table_names() {
        "cat_mood"
    } else if api.is_mysql() {
        "Cat_mood"
    } else {
        "CatMood"
    };

    engine
        .assert_schema()
        .assert_enum(enum_name, |enm| enm.assert_values(&["HUNGRY"]))
        .unwrap();

    let dm2 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HUNGRY
            HAPPY
            JOYJOY
        }
    "#;

    engine.schema_push(dm2).send_sync().unwrap().assert_green().unwrap();

    engine
        .assert_schema()
        .assert_enum(enum_name, |enm| enm.assert_values(&["HUNGRY", "HAPPY", "JOYJOY"]))
        .unwrap();
}

#[test_connector(capabilities(Enums))]
fn variants_can_be_removed_from_an_existing_enum(api: TestApi) {
    let engine = api.new_engine();

    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    engine.schema_push(dm1).send_sync().unwrap().assert_green().unwrap();

    let enum_name = if api.lower_cases_table_names() {
        "cat_mood"
    } else if api.is_mysql() {
        "Cat_mood"
    } else {
        "CatMood"
    };

    engine
        .assert_schema()
        .assert_enum(enum_name, |enm| enm.assert_values(&["HAPPY", "HUNGRY"]))
        .unwrap();

    let dm2 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HUNGRY
        }
    "#;

    let warning = if api.is_mysql() {
        "The values [HAPPY] on the enum `Cat_mood` will be removed. If these variants are still used in the database, this will fail."
    } else {
        "The values [HAPPY] on the enum `CatMood` will be removed. If these variants are still used in the database, this will fail."
    };

    engine
        .schema_push(dm2)
        .force(true)
        .send_sync()
        .unwrap()
        .assert_warnings(&[warning.into()])
        .unwrap()
        .assert_executable()
        .unwrap();

    engine
        .assert_schema()
        .assert_enum(enum_name, |enm| enm.assert_values(&["HUNGRY"]))
        .unwrap();
}

#[test_connector(capabilities(Enums))]
fn models_with_enum_values_can_be_dropped(api: TestApi) {
    let engine = api.new_engine();
    engine
        .schema_push(BASIC_ENUM_DM)
        .send_sync()
        .unwrap()
        .assert_green()
        .unwrap();

    engine.assert_schema().assert_tables_count(1).unwrap();

    engine.insert("Cat").value("id", 1).value("mood", "HAPPY").result_raw();

    let warn = if api.lower_cases_table_names() {
        "You are about to drop the `cat` table, which is not empty (1 rows)."
    } else {
        "You are about to drop the `Cat` table, which is not empty (1 rows)."
    };

    engine
        .schema_push("")
        .force(true)
        .send_sync()
        .unwrap()
        .assert_executable()
        .unwrap()
        .assert_warnings(&[warn.into()])
        .unwrap();

    engine.assert_schema().assert_tables_count(0).unwrap();
}

#[test_connector(capabilities(Enums))]
fn enum_field_to_string_field_works(api: TestApi) {
    let engine = api.new_engine();
    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood?
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    engine.schema_push(dm1).send_sync().unwrap().assert_green().unwrap();

    engine
        .assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("mood", |col| col.assert_type_is_enum())
        })
        .unwrap();

    engine.insert("Cat").value("id", 1).value("mood", "HAPPY").result_raw();

    let dm2 = r#"
        model Cat {
            id      Int @id
            mood    String?
        }
    "#;

    engine
        .schema_push(dm2)
        .force(true)
        .send_sync()
        .unwrap()
        .assert_executable()
        .unwrap();

    engine
        .assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("mood", |col| col.assert_type_is_string())
        })
        .unwrap();
}

#[test_connector(capabilities(Enums))]
fn string_field_to_enum_field_works(api: TestApi) {
    let engine = api.new_engine();
    let dm1 = r#"
        model Cat {
            id      Int @id
            mood    String?
        }
    "#;

    engine.schema_push(dm1).send_sync().unwrap().assert_green().unwrap();

    engine
        .assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("mood", |col| col.assert_type_is_string())
        })
        .unwrap();

    engine.insert("Cat").value("id", 1).value("mood", "HAPPY").result_raw();

    let dm2 = r#"
        model Cat {
            id Int @id
            mood CatMood?
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    let warn = if api.is_postgres() {
        "The `mood` column on the `Cat` table would be dropped and recreated. This will lead to data loss."
    } else if api.lower_cases_table_names() {
        "You are about to alter the column `mood` on the `cat` table, which contains 1 non-null values. The data in that column will be cast from `VarChar(191)` to `Enum(\"Cat_mood\")`."
    } else {
        "You are about to alter the column `mood` on the `Cat` table, which contains 1 non-null values. The data in that column will be cast from `VarChar(191)` to `Enum(\"Cat_mood\")`."
    };

    engine
        .schema_push(dm2)
        .force(true)
        .send_sync()
        .unwrap()
        .assert_executable()
        .unwrap()
        .assert_warnings(&[warn.into()])
        .unwrap();

    engine
        .assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("mood", |col| col.assert_type_is_enum())
        })
        .unwrap();
}

#[test_connector(capabilities(Enums))]
fn enums_used_in_default_can_be_changed(api: TestApi) {
    let engine = api.new_engine();

    let dm1 = r#"
        model Panther {
            id Int @id
            mood CatMood @default(HAPPY)
        }

        model Tiger {
            id Int @id
            mood CatMood @default(HAPPY)
        }

         model Leopard {
            id Int @id
            mood CatMood @default(HAPPY)
        }

        model Lion {
            id Int @id
            mood CatMood
        }

        model GoodDog {
            id Int @id
            mood DogMood @default(HAPPY)
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }

        enum DogMood {
            HAPPY
            HUNGRY
        }
    "#;

    engine.schema_push(dm1).send_sync().unwrap().assert_green().unwrap();

    engine.assert_schema().assert_tables_count(5).unwrap();

    let dm2 = r#"
        model Panther {
            id Int @id
            mood CatMood @default(HAPPY)
        }

        model Tiger {
            id Int @id
            mood CatMood @default(HAPPY)
        }

         model Leopard {
            id Int @id
            mood CatMood
        }

        model Lion {
            id Int @id
            mood CatMood @default(HAPPY)
        }

        model GoodDog {
            id Int @id
            mood DogMood @default(HAPPY)
        }

        enum CatMood {
            HAPPY
            ANGRY
        }

        enum DogMood {
            HAPPY
            HUNGRY
            SLEEPY
        }
    "#;

    if api.is_postgres() {
        engine.schema_push(dm2)
            .force(true)
            .send_sync()
            .unwrap()
            .assert_executable().unwrap()
            .assert_warnings(&["The values [HUNGRY] on the enum `CatMood` will be removed. If these variants are still used in the database, this will fail.".into()]
            ).unwrap();
    } else {
        engine.schema_push(dm2)
            .force(true)
            .send_sync().unwrap()
            .assert_executable().unwrap()
            .assert_warnings(& ["The values [HUNGRY] on the enum `Panther_mood` will be removed. If these variants are still used in the database, this will fail.".into(),
                "The values [HUNGRY] on the enum `Tiger_mood` will be removed. If these variants are still used in the database, this will fail.".into(),]
            ).unwrap();
    };

    engine.assert_schema().assert_tables_count(5).unwrap();
}

#[test_connector(capabilities(Enums))]
fn changing_all_values_of_enums_used_in_defaults_works(api: TestApi) {
    let api = api.new_engine();
    let dm1 = r#"
        model Cat {
            id Int @id
            morningMood             CatMood @default(HUNGRY)
            alternateMorningMood    CatMood @default(HUNGRY)
            afternoonMood           CatMood @default(HAPPY)
            eveningMood             CatMood @default(HUNGRY)
            defaultMood             CatMood
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push(dm1).send_sync().unwrap().assert_green().unwrap();

    let dm2 = r#"
        model Cat {
            id Int @id
            morningMood             CatMood @default(MEOW)
            alternateMorningMood    CatMood @default(MEOWMEOWMEOW)
            afternoonMood           CatMood @default(PURR)
            eveningMood             CatMood @default(MEOWMEOW)
            defaultMood             CatMood
        }

        enum CatMood {
            MEOW
            MEOWMEOW
            MEOWMEOWMEOW
            PURR
        }
    "#;

    api.schema_push(dm2).force(true).send_sync().unwrap();

    api.assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("eveningMood", |col| Ok(col.assert_enum_default("MEOWMEOW")))
        })
        .unwrap();
}
