# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/twine-protocol/twine-rs/releases/tag/twine_lib-v0.1.0) - 2025-03-13

### Added

- [**breaking**] Twine.strand and Twine.tixel return references
- [**breaking**] breaking refactor of Tixel and Strand to use Arc internally
- add helper methods for ResolverSetSeries for dyn boxed resolvers
- expose drop_index in tixel
- add len() to CrossStitches
- method to retrieve strands as set from crossstitches
- add extract_details convenience method
- helper functions for stitch inclusion check
- improve implementation of conversion to tagged
- add qol methods to update stitches
- implement into query for other int types
- add Strand variant to AnyQuery
- [**breaking**] rename Query to SingleQuery
- implement resolver for AsRef<dyn BaseResolver>
- add save_sync to memory store
- Implement FromStr for Specification
- [**breaking**] conditional Send for Resolver/Store to allow for wasm
- add Tagged for sending twine data via dag_json
- [**breaking**] more strict string parsing of queries
- memory cache for resolvers
- feature flags for hash functions
- [**breaking**] rework resolver to return resolutions which validate query

### Fixed

- [**breaking**] change name from resolve_and_add to add_or_refresh
- cross_stitches of length 0 incorrectly validated
- [**breaking**] Encoded cross-stitches could have arbitrary order
- RSA keys with different modulus sizes not correctly parsed
- [**breaking**] change payload extraction error type
- [**breaking**] change language to unchecked_base so not imply unsafe rust
- Tagged<AnyTwine> serialization incorrect
- [**breaking**] breaking change. dag_json methods are now tagged_dag_json
- hash features in meta package
- bug with query string parse
- batched range queries failing on ranges near batch size
- [**breaking**] reorganize and adjust BaseResolver to discourage client usage
- bug with serde_dag_json and newtypes
- [**breaking**] use uppercase for key algorithm identifiers in v2
- fixes and checks for queries and range queries
- fixes for resolving
- fixes to range queries and serialization
- fix signer/verifier leak
- fixes for store and http store
- fix for edge case
- fixes and refactor Store trait
- fix ranges, added buffer option

### Other

- prepare release-please workflow
- leverage workspace dependency management
- fix ci workflow
- run rust fmt
- remove unused "use"
- cleanup old code
- update test for resolver set
- refactor internals for simplicity
- Update public_key.rs
- add from_car_bytes helper function
- update deps
- Update resolution.rs
- cleanup todo comments
- test for invalid key type in key field
- fix test cases
- add min rust version
- added twine creation and store config to cli
- require explicit implementation of resolver for optimizations
- make resolver set explicit instead of implicit vec
- revert range start end methods. make properties public
- add spec_str access
- rework to use serde_bytes for encoding
- implement v2 builder
- implemented a version 2
- wip on v2 spec
- finally finished refactor
- restrict deserialization, remove redundant verify
- huge refactor to prepare for v2
- add build feature for builder in top level
- return references for payload and details, impl extract_payload()
- allow range syntax of cid:latest:n
- serialize cids as strings in config file
- cleanup
- impl Eq, Hash, PartialEq for twine structs
- api fix. previous should be option
- estimate size of strands
- cli list fix for missing latest
- rework of range resolution to allow for bidirectional ranges
- rework range query to allow increasing iteration
- added sync capability
- attempt to implement cache
- cleanup
- Update serialization.rs
- migrate to ipld_core
- add has_index method to resolver
- multi resolver fix
- towards local store in twine_cli
- cleanup
- refactor resolver again. Nicer implementation for Box<dyn BaseResolver>
- refactor resolver trait
- change resolve_index to use negative indices too
- string conversion for queries
- name fix
- pulsecontent with payload type
- use u8 for radix
- skiplist utils
- stitch refresh method
- remove need for alg in key
- better signature check
- move specification error
- bytes method in twinecontent trait
- CrossStitches type
- resolver Latest query from strand Cid
- reorganize
- reorganize
- implement store for http resolver
- added car encoder
- change store definition and implement for cache
- minor fix
- added memory cache
- add strands list to resolver, and test memory store
- refactor tests
- added store and memory store
- range stream works miraculously
- ease of use for dagjson decode
- range stream works
- prep for range queries
- added resolver trait and simple http resolver
- radix
- handle version and subspec
- added twine structure for main usage
- AnyTwine is either strand or tixel
- better validation handling and better errors
- add stitches abstraction
- simplify payload stuff
- refactor spec handling, add payload unpack
- reorganize, use custom error with thiserror
- Update lib.rs
- signature verification works
- rename things
- correct serialization and deserialization with cid checks
- serde-cbor fix
- begin huge refactor
- Create blanket impls for common ser/de
- Clean up todos
- Clean up imports to use extern'ed libipld and josekit
- Verify that previous mixin order is preserved
- to/from JSON strings
- re-export libipld per https://github.com/rust-lang/api-guidelines/discussions/176
- frame out next steps
- make things base64 encoded
- Squash bugs
- Convert to using thiserror
- Make verification friendly for first pulses
- refactor to use verification
- Bug fixes
- Clean up twine_code tests
- Squash type bugs
- Debugging
- Update examples builder examples
- Add hasher_of()
- Configure new twine_builder lib
- use jws signer
- Clean up examples
- Flesh out chain builder
- Add chain builder
- Add examples
- Make twine functions public
- Refactor core twine
- Add jose dep + framework signing
- Add twine base structs
- Some sparse examples
- Define the chain creation interface (has :bug:s)
- add deps
- setup initial example structure
