# twine_sql_store

[![Crates.io Version](https://img.shields.io/crates/v/twine_sql_store)](https://crates.io/crates/twine_sql_store)
[![docs.rs (with version)](https://img.shields.io/docsrs/twine_sql_store/latest)](https://docs.rs/twine_sql_store/latest/twine_sql_store/)

A [`twine_lib::store::Store`] implementation that saves twine data to
an SQL database. The sql support must be enabled through feature flags.

Currently supported via feature flags:

- `sqlite`
- `mysql`

## Database setup

### Sqlite

For sqlite, one can either use the [`crate::sqlite::SCHEMA`] string for
sql that sets up the tables, or call [`crate::sqlite::SqliteStore::create_tables()`]

### Mysql

See the file [./schemas/mysql/001_init-mysql.sql] for a schema to use with your
mysql database.
