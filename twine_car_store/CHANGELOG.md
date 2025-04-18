# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/twine-protocol/twine-rs/compare/twine_car_store-v0.1.2...twine_car_store-v0.1.3) - 2025-04-18

### Other

- updated the following local packages: twine_lib, twine_builder

## [0.1.2](https://github.com/twine-protocol/twine-rs/compare/twine_car_store-v0.1.1...twine_car_store-v0.1.2) - 2025-03-26

### Other

- updated the following local packages: twine_lib

## [0.1.1](https://github.com/twine-protocol/twine-rs/compare/twine_car_store-v0.1.0...twine_car_store-v0.1.1) - 2025-03-21

### Other

- *(docs)* add missing example doc comment
- *(docs)* document twine_car_store

## [0.1.0](https://github.com/twine-protocol/twine-rs/releases/tag/twine_car_store-v0.1.0) - 2025-03-13

### Added

- [**breaking**] breaking refactor of Tixel and Strand to use Arc internally
- [**breaking**] car store new() is now sync

### Fixed

- flush car data periodically
- only save car on change
- handle non-existant car file on startup

### Other

- prepare release-please workflow
- update deps
- leverage workspace dependency management
- run rust fmt
- Update local-car-file.rs
- add twine_car_store
