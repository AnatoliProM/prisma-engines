default:
	cargo build

# Build the crates with deny-warnings on to emulate CI
pedantic:
	RUSTFLAGS="-D warnings" cargo build

release:
	cargo build --release

dev-sqlite:
	cp dev-configs/sqlite.yml prisma.yml
	echo 'sqlite' > current_connector

dev-postgres:
	docker-compose -f docker-compose/dev-postgres.yml up -d --remove-orphans
	cp dev-configs/postgres.yml prisma.yml
	echo 'postgres' > current_connector

dev-mysql:
	docker-compose -f docker-compose/dev-mysql.yml up -d --remove-orphans
	cp dev-configs/mysql.yml prisma.yml
	echo 'mysql' > current_connector

dev-all:
	docker-compose -f .buildkite/engine-build-cli/docker-test-setups/docker-compose.test.all.yml up -d --remove-orphans

use-local-migration-engine:
	cargo build --release
	cp target/release/migration-engine $(PRISMA2_BINARY_PATH)/

use-local-query-engine:
	cargo build --release
	cp target/release/prisma $(PRISMA2_BINARY_PATH)/runtime/
	cp target/release/prisma $(PRISMA2_BINARY_PATH)/query-engine-darwin
