# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/twine-protocol/twine-rs/compare/twine_builder-v0.1.0...twine_builder-v0.1.1) - 2025-03-21

### Fixed

- *(twine_builder)* expose underlying builder types

### Other

- *(docs)* specify feature flags needed
- *(docs)* document twine_builder

## [0.1.0](https://github.com/twine-protocol/twine-rs/releases/tag/twine_builder-v0.1.0) - 2025-03-13

### Added

- [**breaking**] Twine.strand and Twine.tixel return references
- [**breaking**] breaking refactor of Tixel and Strand to use Arc internally
- implement drop stitches in v2 builder
- add error variant for payload construction
- add builder method accepting payload build function
- add qol methods to update stitches
- [**breaking**] convert from and to PEM format
- add genesis time customization for builder
- re-export pkcs8 in twine_builder
- feature flags for hash functions

### Fixed

- builder_v2 contained noop cross stitches method
- [**breaking**] builder payload default differed between first and next
- [**breaking**] change twine builder signature to use const generics
- make biscuit dep optional
- [**breaking**] change name from resolve_and_add to add_or_refresh
- cross-stitches weren't carrying through from previous
- RSA keys with different modulus sizes not correctly parsed
- rsa usage wasn't behind rsa feature
- tixel builder will copy specification from strand
- [**breaking**] breaking change. dag_json methods are now tagged_dag_json
- fix signer/verifier leak

### Other

- prepare release-please workflow
- update deps
- leverage workspace dependency management
- fix ci workflow
- run rust fmt
- generic label update
- minor edit
- refactor internals for simplicity
- added quick implementation of pickledb store
- update thiserror
- add min rust version
- added twine creation and store config to cli
- builder uses borrowed previous twines
- deprecate biscuit signer
- implemented ring signer
- rework to use serde_bytes for encoding
- implement v2 builder
- wip on v2 spec
- reorganize and fix examples
- finally finished refactor
- huge refactor to prepare for v2
- allow general struct as strand details too
- return references for payload and details, impl extract_payload()
- migrate to ipld_core
- refactor resolver trait
- Update simple.rs
- some fixes
- rename builder
- builder implemented
- implement strand builder
- update package
- Clean up todos
- Clean up imports to use extern'ed libipld and josekit
- use linked hash map for Pulse/ChainBuilder
- Add doc comments to clarify behavior of mixin and mixins
- make chain/pulse builder update mixins
- try building a web api
- frame out next steps
- make things base64 encoded
- Squash bugs
- Convert to using thiserror
- Make verification friendly for first pulses
- refactor to use verification
- rename pulse builder
- Bug fixes
- Add pretty printing to creation examples
- Clean up twine_code tests
- Compiled and working builder examples!
- Use borrowed chain and pulse in PulseBuilder
- Refactor example types
- Squash type bugs
- Debugging
- Update examples builder examples
- Add hasher_of()
- update builders
- Configure new twine_builder lib
