use indoc::indoc;
use introspection_engine_tests::test_api::*;

// referentialIntegrity = "prisma" preserves the relation policy ("prisma") as well as @relations.
#[test_connector(tags(Sqlite))]
async fn referential_integrity_prisma(api: &TestApi) -> TestResult {
    let init = formatdoc! {r#"
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            "bar_id" INTEGER NOT NULL
        );
        
        CREATE TABLE "Bar" (
            "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT
        );
        
        CREATE UNIQUE INDEX "Foo_bar_id_key" ON "Foo"("bar_id");
    "#};

    api.raw_cmd(&init).await;

    let input = indoc! {r#"
        generator client {
            provider        = "prisma-client-js"
            previewFeatures = ["referentialIntegrity"]
        }

        datasource db {
            provider             = "sqlite"
            url                  = env("TEST_DATABASE_URL")
            referentialIntegrity = "prisma"
        }

        model Foo {
            id     Int @id
            bar    Bar @relation(fields: [bar_id], references: [id])
            bar_id Int @unique
        }

        model Bar {
            id  Int  @id
            foo Foo?
        }
    "#};

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["referentialIntegrity"]
        }

        datasource db {
          provider             = "sqlite"
          url                  = env("TEST_DATABASE_URL")
          referentialIntegrity = "prisma"
        }

        model Foo {
          id     Int @id @default(autoincrement())
          bar    Bar @relation(fields: [bar_id], references: [id])
          bar_id Int @unique
        }

        model Bar {
          id  Int  @id @default(autoincrement())
          foo Foo?
        }
    "#]];

    let result = api.re_introspect_config(input).await?;
    expected.assert_eq(&result);

    Ok(())
}

// referentialIntegrity = "foreignKeys" preserves the relation policy ("foreignKeys") as well as @relations, which are moved to the bottom.
#[test_connector(tags(Sqlite))]
async fn referential_integrity_foreign_keys(api: &TestApi) -> TestResult {
    let init = formatdoc! {r#"
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            "bar_id" INTEGER NOT NULL,
            CONSTRAINT "Foo_bar_id_fkey" FOREIGN KEY ("bar_id") REFERENCES "Bar" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
        );
        
        CREATE TABLE "Bar" (
            "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT
        );
        
        CREATE UNIQUE INDEX "Foo_bar_id_key" ON "Foo"("bar_id");
    "#};

    api.raw_cmd(&init).await;

    let input = indoc! {r#"
        generator client {
            provider        = "prisma-client-js"
            previewFeatures = ["referentialIntegrity"]
        }

        datasource db {
            provider             = "sqlite"
            url                  = env("TEST_DATABASE_URL")
            referentialIntegrity = "foreignKeys"
        }

        model Foo {
            id     Int @id
            bar    Bar @relation(fields: [bar_id], references: [id])
            bar_id Int @unique
        }

        model Bar {
            id  Int  @id
            foo Foo?
        }
    "#};

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["referentialIntegrity"]
        }

        datasource db {
          provider             = "sqlite"
          url                  = env("TEST_DATABASE_URL")
          referentialIntegrity = "foreignKeys"
        }

        model Foo {
          id     Int @id @default(autoincrement())
          bar_id Int @unique
          bar    Bar @relation(fields: [bar_id], references: [id])
        }

        model Bar {
          id  Int  @id @default(autoincrement())
          foo Foo?
        }
    "#]];

    let result = api.re_introspect_config(input).await?;
    expected.assert_eq(&result);

    Ok(())
}

// relationMode = "prisma" preserves the relation policy ("prisma") as well as @relations.
#[test_connector(tags(Sqlite))]
async fn relation_mode_prisma(api: &TestApi) -> TestResult {
    let init = formatdoc! {r#"
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            "bar_id" INTEGER NOT NULL
        );
        
        CREATE TABLE "Bar" (
            "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT
        );
        
        CREATE UNIQUE INDEX "Foo_bar_id_key" ON "Foo"("bar_id");
    "#};

    api.raw_cmd(&init).await;

    let input = indoc! {r#"
        generator client {
            provider        = "prisma-client-js"
            previewFeatures = ["referentialIntegrity"]
        }

        datasource db {
            provider     = "sqlite"
            url          = env("TEST_DATABASE_URL")
            relationMode = "prisma"
        }

        model Foo {
            id     Int @id
            bar    Bar @relation(fields: [bar_id], references: [id])
            bar_id Int @unique
        }

        model Bar {
            id  Int  @id
            foo Foo?
        }
    "#};

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["referentialIntegrity"]
        }

        datasource db {
          provider     = "sqlite"
          url          = env("TEST_DATABASE_URL")
          relationMode = "prisma"
        }

        model Foo {
          id     Int @id @default(autoincrement())
          bar    Bar @relation(fields: [bar_id], references: [id])
          bar_id Int @unique
        }

        model Bar {
          id  Int  @id @default(autoincrement())
          foo Foo?
        }
    "#]];

    let result = api.re_introspect_config(input).await?;
    expected.assert_eq(&result);

    Ok(())
}

// relationMode = "foreignKeys" preserves the relation policy ("foreignKeys") as well as @relations, which are moved to the bottom.
#[test_connector(tags(Sqlite))]
async fn relation_mode_foreign_keys(api: &TestApi) -> TestResult {
    let init = formatdoc! {r#"
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            "bar_id" INTEGER NOT NULL,
            CONSTRAINT "Foo_bar_id_fkey" FOREIGN KEY ("bar_id") REFERENCES "Bar" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
        );
        
        CREATE TABLE "Bar" (
            "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT
        );
        
        CREATE UNIQUE INDEX "Foo_bar_id_key" ON "Foo"("bar_id");
    "#};

    api.raw_cmd(&init).await;

    let input = indoc! {r#"
        generator client {
            provider        = "prisma-client-js"
            previewFeatures = ["referentialIntegrity"]
        }

        datasource db {
            provider     = "sqlite"
            url          = env("TEST_DATABASE_URL")
            relationMode = "foreignKeys"
        }

        model Foo {
            id     Int @id
            bar    Bar @relation(fields: [bar_id], references: [id])
            bar_id Int @unique
        }

        model Bar {
            id  Int  @id
            foo Foo?
        }
    "#};

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["referentialIntegrity"]
        }

        datasource db {
          provider     = "sqlite"
          url          = env("TEST_DATABASE_URL")
          relationMode = "foreignKeys"
        }

        model Foo {
          id     Int @id @default(autoincrement())
          bar_id Int @unique
          bar    Bar @relation(fields: [bar_id], references: [id])
        }

        model Bar {
          id  Int  @id @default(autoincrement())
          foo Foo?
        }
    "#]];

    let result = api.re_introspect_config(input).await?;
    expected.assert_eq(&result);

    Ok(())
}

// @relations are moved to the bottom of the model even when no referentialIntegrity/relationMode is used.
#[test_connector(tags(Sqlite))]
async fn no_relation_mode(api: &TestApi) -> TestResult {
    let init = formatdoc! {r#"
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            "bar_id" INTEGER NOT NULL,
            CONSTRAINT "Foo_bar_id_fkey" FOREIGN KEY ("bar_id") REFERENCES "Bar" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
        );
        
        CREATE TABLE "Bar" (
            "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT
        );
        
        CREATE UNIQUE INDEX "Foo_bar_id_key" ON "Foo"("bar_id");
    "#};

    api.raw_cmd(&init).await;

    let input = indoc! {r#"
        datasource db {
            provider = "sqlite"
            url      = env("TEST_DATABASE_URL")
        }

        model Foo {
            id     Int @id
            bar    Bar @relation(fields: [bar_id], references: [id])
            bar_id Int @unique
        }

        model Bar {
            id  Int  @id
            foo Foo?
        }
    "#};

    let expected = expect![[r#"
        datasource db {
          provider = "sqlite"
          url      = env("TEST_DATABASE_URL")
        }

        model Foo {
          id     Int @id @default(autoincrement())
          bar_id Int @unique
          bar    Bar @relation(fields: [bar_id], references: [id])
        }

        model Bar {
          id  Int  @id @default(autoincrement())
          foo Foo?
        }
    "#]];

    let result = api.re_introspect_config(input).await?;
    expected.assert_eq(&result);

    Ok(())
}
