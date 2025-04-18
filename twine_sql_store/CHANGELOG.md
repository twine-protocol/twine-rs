# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/twine-protocol/twine-rs/compare/twine_sql_store-v0.1.2...twine_sql_store-v0.1.3) - 2025-04-18

### Other

- updated the following local packages: twine_lib, twine_builder

## [0.1.2](https://github.com/twine-protocol/twine-rs/compare/twine_sql_store-v0.1.1...twine_sql_store-v0.1.2) - 2025-03-26

### Other

- updated the following local packages: twine_lib

## [0.1.1](https://github.com/twine-protocol/twine-rs/compare/twine_sql_store-v0.1.0...twine_sql_store-v0.1.1) - 2025-03-21

### Other

- Update sqlite-in-memory.rs
- *(docs)* document twine_sql_store

## [0.1.0](https://github.com/twine-protocol/twine-rs/releases/tag/twine_sql_store-v0.1.0) - 2025-03-13

### Added

- [**breaking**] breaking refactor of Tixel and Strand to use Arc internally

### Fixed

- mysql store had incorrect range retrieval limits
- sql store range resolve was broken
- add async runtime feature flags
- non-exhaustive pattern match for sql store
- fixes preventing out of order saves in sql store

### Other

- prepare release-please workflow
- leverage workspace dependency management
- run rust fmt
- huge refactor of sql store to get it working with mysql
- Update lib.rs
- added sql_store
