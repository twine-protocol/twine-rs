# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2](https://github.com/twine-protocol/twine-rs/compare/twine_pickledb_store-v0.1.1...twine_pickledb_store-v0.1.2) - 2025-03-26

### Other

- updated the following local packages: twine_lib

## [0.1.1](https://github.com/twine-protocol/twine-rs/compare/twine_pickledb_store-v0.1.0...twine_pickledb_store-v0.1.1) - 2025-03-21

### Other

- *(docs)* document twine_pickledb_store

## [0.1.0](https://github.com/twine-protocol/twine-rs/releases/tag/twine_pickledb_store-v0.1.0) - 2025-03-13

### Added

- [**breaking**] breaking refactor of Tixel and Strand to use Arc internally
- impl Clone and Debug for pickledbstore

### Fixed

- flush pickle data on every save
- *(pickledb_store)* incorrect handling of empty lists

### Other

- prepare release-please workflow
- update deps
- leverage workspace dependency management
- run rust fmt
- Update lib.rs
- added quick implementation of pickledb store
