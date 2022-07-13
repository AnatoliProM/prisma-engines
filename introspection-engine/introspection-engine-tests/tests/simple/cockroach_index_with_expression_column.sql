-- tags=CockroachDb

CREATE TABLE communication_channels (
    id bigint NOT NULL,
    path character varying(255) NOT NULL,
    path_type character varying(255) DEFAULT 'email'::character varying NOT NULL,
    position integer,
    user_id bigint NOT NULL,
    pseudonym_id bigint,
    bounce_count integer DEFAULT 0,
    confirmation_code character varying(255)
);

/*
now create the indexes with expression columns
*/

CREATE INDEX index_communication_channels_on_path_and_path_type ON communication_channels (lower((path)::text), path_type);
CREATE UNIQUE INDEX index_communication_channels_on_user_id_and_path_and_path_type ON communication_channels (user_id, lower((path)::text), path_type);
CREATE INDEX index_communication_channels_on_confirmation_code ON communication_channels (confirmation_code);

/*
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "cockroachdb"
  url      = "env(TEST_DATABASE_URL)"
}

model communication_channels {
  id                BigInt
  path              String  @db.String(255)
  path_type         String  @default("email") @db.String(255)
  position          Int?
  user_id           BigInt
  pseudonym_id      BigInt?
  bounce_count      Int?    @default(0)
  confirmation_code String? @db.String(255)

  @@unique([user_id, path_type], map: "index_communication_channels_on_user_id_and_path_and_path_type")
  @@index([confirmation_code, path_type], map: "index_communication_channels_on_confirmation_code")
}
*/
