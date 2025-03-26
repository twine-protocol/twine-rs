# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2](https://github.com/twine-protocol/twine-rs/compare/twine_http_store-v0.1.1...twine_http_store-v0.1.2) - 2025-03-26

### Fixed

- *(docs)* fix links to other crates

## [0.1.1](https://github.com/twine-protocol/twine-rs/compare/twine_http_store-v0.1.0...twine_http_store-v0.1.1) - 2025-03-21

### Fixed

- *(twine_http_store)* batch streams when saving

### Other

- *(docs)* document twine_http_store

## [0.1.0](https://github.com/twine-protocol/twine-rs/releases/tag/twine_http_store-v0.1.0) - 2025-03-13

### Added

- [**breaking**] breaking refactor of Tixel and Strand to use Arc internally
- [**breaking**] rename Query to SingleQuery
- [**breaking**] conditional Send for Resolver/Store to allow for wasm
- [**breaking**] remove "Registration" flow for http v2 api. batch saves.
- add Tagged for sending twine data via dag_json
- memory cache for resolvers
- [**breaking**] rework resolver to return resolutions which validate query

### Fixed

- *(twine_http_store)* batch stream saving
- [**breaking**] breaking change, remove timeout option
- [**breaking**] change language to unchecked_base so not imply unsafe rust
- Tagged<AnyTwine> serialization incorrect
- [**breaking**] breaking change. dag_json methods are now tagged_dag_json
- include error response message from html body
- [**breaking**] reorganize and adjust BaseResolver to discourage client usage
- fixes for resolving
- fixes to range queries and serialization
- fix for http resolver not implementing HEAD yet
- fix renaming

### Other

- prepare release-please workflow
- leverage workspace dependency management
- run rust fmt
- refactor internals for simplicity
- incorrect return type
- Update Cargo.toml
- Update simple.rs
- Update v2.rs
- internal refactor to use from_car_bytes
- Update simple_v2.rs
- Create copy_between.rs
- update deps
- add min rust version
- require explicit implementation of resolver for optimizations
- Update lib.rs
- Update lib.rs
- use backon to retry http requests upon certain errors
- rework of range resolution to allow for bidirectional ranges
- attempt to implement cache
- cleanup
- migrate to ipld_core
- add has_index method to resolver
- rename buffer_size to concurrency
- cleanup
- refactor resolver again. Nicer implementation for Box<dyn BaseResolver>
- refactor resolver trait
- rename http_resolver to http_store
