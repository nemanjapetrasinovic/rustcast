GIT_VERSION:=$(shell git describe --abbrev=8 | sed 's/-/./')
DB_PATH=${HOME}/.rustcast.db
DATABASE_URL_BASE=sqlite://${DB_PATH}
DATABASE_URL=sqlite://${HOME}/.rustcast.db?mode=rwc

build: Cargo.toml build-migration
	git describe
	sed -i '' -e 's/^version =.*/version = "$(GIT_VERSION)"/' Cargo.toml
	cargo build --release

build-migration:
	make -C migrations build

migrate:
	DATABASE_URL="$(DATABASE_URL_BASE)?mode=rwc" make -C migrations migrate

migrate-fresh:
	DATABASE_URL="$(DATABASE_URL_BASE)?mode=rwc" make -C migrations fresh

generate-entity:
	sea-orm-cli generate entity -u $(DATABASE_URL_BASE) -o src/entity

sql:
	sqlite3 $(DB_PATH)
