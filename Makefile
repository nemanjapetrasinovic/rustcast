GIT_VERSION:=$(shell git describe --abbrev=8 | sed 's/-/./')
UNAME:=$(shell uname -s)

ifeq ($(OS),Windows_NT)
	DB_PATH=$(USERPROFILE)/.rustcast.db
else ifeq ($(UNAME),Linux)
	DB_PATH=$(HOME)/.rustcast.db
else ifeq ($(UNAME),Darwin)
	DB_PATH=$(HOME)/.rustcast.db
else
	DB_PATH=$(HOME)/.rustcast.db
endif

DATABASE_URL_BASE=sqlite://$(DB_PATH)
DATABASE_URL=sqlite://$(DB_PATH)?mode=rwc

build: Cargo.toml build-migration
	git describe
ifeq ($(UNAME),Linux)
	sed -i 's/^version =.*/version = "$(GIT_VERSION)"/' Cargo.toml
else ifeq ($(UNAME),Darvin)
	sed -i '' -e 's/^version =.*/version = "$(GIT_VERSION)"/' Cargo.toml
endif
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
