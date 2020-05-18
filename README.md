# Substrate Archive Node

Run this alongside the substrate client to sync all historical TxData. Allows
you to run queries on a PostgreSQL database.

The schema for the PostgreSQL database is described in the Pdf File at the root of this directory



## Required External Dependencies
- PostgreSQL

### Developing
Init script (`./scripts/init.sh`) should install all dependencies and setup a default database user for you. Just make sure to change the password of user `archive` via psql in a production or security-sensitive environment.

Required Dependencies:
Ubuntu: `postgresql`, `postgresql-contrib`, `libpq-dev`
Fedora: `postgresql`, `postgresql-contrib`, `postgresql-devel`
Regardless of Distribution (for development):
Rust: `diesel_cli`
	- install with: `cargo install diesel_cli --no-default-features --features postgres` to avoid installing MySQL dependencies


To create all tables, use the command `diesel migration run`

