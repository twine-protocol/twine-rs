# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2](https://github.com/twine-protocol/twine-rs/compare/twine_cli-v0.1.1...twine_cli-v0.1.2) - 2025-03-26

### Other

- updated the following local packages: twine_lib, twine_http_store

## [0.1.1](https://github.com/twine-protocol/twine-rs/compare/twine_cli-v0.1.0...twine_cli-v0.1.1) - 2025-03-21

### Other

- *(docs)* document twine_cli

## [0.1.0](https://github.com/twine-protocol/twine-rs/releases/tag/twine_cli-v0.1.0) - 2025-03-13

### Added

- [**breaking**] Twine.strand and Twine.tixel return references
- [**breaking**] breaking refactor of Tixel and Strand to use Arc internally
- *(cli)* add rsa as option for keygen
- [**breaking**] rename Query to SingleQuery
- *(cli)* better ctrl-c management when syncing
- *(cli)* determine store type by extension
- feat!(cli): Full cli rework
- [**breaking**] car store new() is now sync
- *(cli)* allow list command to see car stores
- *(cli)* add keygen, and revamp strand create action
- [**breaking**] rework resolver to return resolutions which validate query

### Fixed

- *(cli)* permission change for keygen only in unix
- *(cli)* change info message text
- *(cli)* add info messages to check cmd
- [**breaking**] change language to unchecked_base so not imply unsafe rust
- [**breaking**] reorganize and adjust BaseResolver to discourage client usage
- fixes and checks for queries and range queries
- fixes for resolving
- fixes to range queries and serialization
- fix cli name
- fixes for concurrent pull
- fixes for progressbars and logging

### Other

- update deps
- leverage workspace dependency management
- run rust fmt
- Create README.md
- add min rust version
- added twine creation and store config to cli
- make resolver set explicit instead of implicit vec
- implemented a version 2
- Update mod.rs
- return references for payload and details, impl extract_payload()
- serialize cids as strings in config file
- cli improvements and fixes for resolver list
- cleanup
- try cli exe name
- Update Cargo.toml
- Update mod.rs
- human readable numbers in progress bar
- estimate size of strands
- Update mod.rs
- cli list fix for missing latest
- rework of range resolution to allow for bidirectional ranges
- added progressbar
- added sync capability
- rename buffer_size to concurrency
- log msg with indices
- towards local store in twine_cli
- cleanup
- refactor resolver again. Nicer implementation for Box<dyn BaseResolver>
- refactor resolver trait
- cli list command
- implement list command
- cli work
- beginning of cli
