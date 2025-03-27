# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2](https://github.com/twine-protocol/twine-rs/compare/twine_sled_store-v0.1.1...twine_sled_store-v0.1.2) - 2025-03-26

### Other

- updated the following local packages: twine_lib

## [0.1.1](https://github.com/twine-protocol/twine-rs/compare/twine_sled_store-v0.1.0...twine_sled_store-v0.1.1) - 2025-03-21

### Other

- *(docs)* document twine_sled_store

## [0.1.0](https://github.com/twine-protocol/twine-rs/releases/tag/twine_sled_store-v0.1.0) - 2025-03-13

### Added

- [**breaking**] breaking refactor of Tixel and Strand to use Arc internally
- use Arc for sled store to prevent conflicts
- [**breaking**] rework resolver to return resolutions which validate query

### Fixed

- sled store key didn't account for smaller cid lengths
- [**breaking**] change language to unchecked_base so not imply unsafe rust
- [**breaking**] reorganize and adjust BaseResolver to discourage client usage
- fixes to range queries and serialization
- fixes for data consistency of latest index. and better stream saving
- fix keys
- fix order of range resolve.
- fixes for range iter

### Other

- update deps
- leverage workspace dependency management
- run rust fmt
- refactor internals for simplicity
- add min rust version
- added twine creation and store config to cli
- require explicit implementation of resolver for optimizations
- revert range start end methods. make properties public
- Update simple.rs
- Update simple.rs
- reorganize and fix examples
- cleanup
- sled save as transaction
- rework of range resolution to allow for bidirectional ranges
- add has_index method to resolver
- test strand retrieval
- cleanup
- cleanup
- refactor resolver again. Nicer implementation for Box<dyn BaseResolver>
- refactor resolver trait
- cli work
- some fixes
- added sled store
