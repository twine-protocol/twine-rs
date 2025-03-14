# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/twine-protocol/twine-rs/releases/tag/twine_protocol-v0.1.0) - 2025-03-13

### Added

- [**breaking**] breaking refactor of Tixel and Strand to use Arc internally

### Fixed

- mysql store had incorrect range retrieval limits
- RSA keys with different modulus sizes not correctly parsed
- hash features in meta package
- fix cli name
- fix renaming

### Other

- rename root package to twine_protocol
- Update Cargo.toml
- rename twine_core to twine_lib
- Update Cargo.toml
- Update Cargo.toml
- Update release-please.yaml
- Create release-plz.yaml
- Update .release-please-manifest.json
- Update .release-please-manifest.json
- *(main)* release 0.1.0
- Update release-please-config.json
- release 0.1.0
- Merge pull request #2 from twine-protocol/release-please--branches--main--components--twine
- Update Cargo.toml
- Update release-please.yaml
- Update ci.yml
- change release-please config
- update release please config
- release please adjustment
- prepare release-please workflow
- update deps
- update deps
- leverage workspace dependency management
- *(docs)* Begin adding documentation
- Update ci.yml
- Update ci.yml
- Update ci.yml
- Update ci.yml
- Update ci.yml
- Update ci.yml
- Update ci.yml
- Update ci.yml
- fix ci workflow
- Create release-please.yaml
- Update ci.yml
- run rust fmt
- Create .rustfmt.toml
- Update ci.yml
- setup ci workflow
- Update README.md
- added sql_store
- added quick implementation of pickledb store
- add twine_car_store
- add randomness strand example
- add http store as feature
- add min rust version
- reorganize and fix examples
- huge refactor to prepare for v2
- add build feature for builder in top level
- try cli exe name
- Update Cargo.toml
- migrate to ipld_core
- beginning of cli
- Update Cargo.toml
- added sled store
- name fix
- rename builder
- Update Cargo.toml
- rename http_resolver to http_store
- reorganize
- add strands list to resolver, and test memory store
- added resolver trait and simple http resolver
- update package
- Update settings.json
- try building a web api
- Configure new twine_builder lib
- use jws signer
- Some sparse examples
- try out github workflow
- setup initial example structure
- Pending changes exported from your codespace
- Initial commit
