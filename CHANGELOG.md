# Changelog

## 0.1.0 (2025-03-13)


### ⚠ BREAKING CHANGES

* Twine.strand and Twine.tixel return references
* breaking refactor of Tixel and Strand to use Arc internally
* builder payload default differed between first and next
* change twine builder signature to use const generics
* change name from resolve_and_add to add_or_refresh
* Encoded cross-stitches could have arbitrary order
* rename Query to SingleQuery
* car store new() is now sync
* convert from and to PEM format
* change payload extraction error type
* conditional Send for Resolver/Store to allow for wasm
* breaking change, remove timeout option
* change language to unchecked_base so not imply unsafe rust
* remove "Registration" flow for http v2 api. batch saves.
* breaking change. dag_json methods are now tagged_dag_json
* more strict string parsing of queries
* rework resolver to return resolutions which validate query
* reorganize and adjust BaseResolver to discourage client usage
* use uppercase for key algorithm identifiers in v2

### Features

* add builder method accepting payload build function ([c025db4](https://github.com/twine-protocol/twine-rs/commit/c025db4aaffa53c051afbb61ae9482e02c911237))
* add error variant for payload construction ([14f76dd](https://github.com/twine-protocol/twine-rs/commit/14f76dd1a9779a1ebfa396fe1602f7633c07adcd))
* add extract_details convenience method ([409df76](https://github.com/twine-protocol/twine-rs/commit/409df762d934c528c23acf6728e13a115d992f72))
* add genesis time customization for builder ([d4337db](https://github.com/twine-protocol/twine-rs/commit/d4337db30a88657247806bd6afb639ddf793fca8))
* add helper methods for ResolverSetSeries for dyn boxed resolvers ([22d4365](https://github.com/twine-protocol/twine-rs/commit/22d43655b81575e269a2e3ac17e4e12a294fab55))
* add len() to CrossStitches ([9c8dbaf](https://github.com/twine-protocol/twine-rs/commit/9c8dbaffbc687af1fe20ce781408cb4321667905))
* add qol methods to update stitches ([34af960](https://github.com/twine-protocol/twine-rs/commit/34af960e3c1f6bfd1ef3a2c58842d9e6f73f06a4))
* add save_sync to memory store ([e032897](https://github.com/twine-protocol/twine-rs/commit/e0328970316d61e76a8d37320c33e6a84e0d5da9))
* add Strand variant to AnyQuery ([52b0c14](https://github.com/twine-protocol/twine-rs/commit/52b0c142f4523e559ac1c9eed721bab1eb344667))
* add Tagged for sending twine data via dag_json ([11a222a](https://github.com/twine-protocol/twine-rs/commit/11a222a2b51d242b1cb89d73160758bab584d9c2))
* breaking refactor of Tixel and Strand to use Arc internally ([5cb9eac](https://github.com/twine-protocol/twine-rs/commit/5cb9eac43fcf0ce2c27bf7ef3afb9a29034000c1))
* car store new() is now sync ([f025064](https://github.com/twine-protocol/twine-rs/commit/f025064aae0243c5eff5ba3cdf525fe9b52812c2))
* **cli:** add keygen, and revamp strand create action ([3e24711](https://github.com/twine-protocol/twine-rs/commit/3e247116b80dfd3bb4549ece2bd0823feca7c420))
* **cli:** add rsa as option for keygen ([1e9d1fc](https://github.com/twine-protocol/twine-rs/commit/1e9d1fc9f93af49a4983260c18685d8eea3fc9f5))
* **cli:** allow list command to see car stores ([ccd0be5](https://github.com/twine-protocol/twine-rs/commit/ccd0be5badec0aeaa37ed0e1c343c6796f61be16))
* **cli:** better ctrl-c management when syncing ([8b0600b](https://github.com/twine-protocol/twine-rs/commit/8b0600bdb5d5070da7bdb0aa7297e9bb03038ccf))
* **cli:** determine store type by extension ([6d3bb20](https://github.com/twine-protocol/twine-rs/commit/6d3bb204c43f5cb4bcc772c91e7eab06140c3159))
* conditional Send for Resolver/Store to allow for wasm ([5ec685d](https://github.com/twine-protocol/twine-rs/commit/5ec685d3356e84e02467d707f24f909458464827))
* convert from and to PEM format ([39335fc](https://github.com/twine-protocol/twine-rs/commit/39335fcd9e00fe0f479142fbbfb6633cdfa2ad15))
* expose drop_index in tixel ([37e257d](https://github.com/twine-protocol/twine-rs/commit/37e257d2b71051abd94fcae5ddacc3d7c8d50530))
* feature flags for hash functions ([ead400b](https://github.com/twine-protocol/twine-rs/commit/ead400b3baa5c6e011f9c57a80f2229aa220ae7c))
* helper functions for stitch inclusion check ([a461c90](https://github.com/twine-protocol/twine-rs/commit/a461c90f72dd72472c98247fe998cfa3b906de64))
* impl Clone and Debug for pickledbstore ([d841d99](https://github.com/twine-protocol/twine-rs/commit/d841d99d4527f9ed463b5b3c90d03e5a8a89bd44))
* implement drop stitches in v2 builder ([26f10c6](https://github.com/twine-protocol/twine-rs/commit/26f10c687fe2f99ba44dd62498129310c44f24eb))
* Implement FromStr for Specification ([cb9a476](https://github.com/twine-protocol/twine-rs/commit/cb9a4768bf559e6d21576739b40588353dfa4ea8))
* implement into query for other int types ([b048fd2](https://github.com/twine-protocol/twine-rs/commit/b048fd2e2950c62a258ed05540aa2ab6faa9c542))
* implement resolver for AsRef&lt;dyn BaseResolver&gt; ([81d4e5c](https://github.com/twine-protocol/twine-rs/commit/81d4e5ccce92046d9fcda0e4547f96527494ba38))
* improve implementation of conversion to tagged ([2d11582](https://github.com/twine-protocol/twine-rs/commit/2d11582dfe5c087bd613fd36ed7adc6df28e72db))
* memory cache for resolvers ([a8cd1ee](https://github.com/twine-protocol/twine-rs/commit/a8cd1eec6444255e997e24a787298d7e7c472bb3))
* method to retrieve strands as set from crossstitches ([37d56a2](https://github.com/twine-protocol/twine-rs/commit/37d56a2f08dd11fb24f9b44a2c0ecf28784b4309))
* more strict string parsing of queries ([6df4027](https://github.com/twine-protocol/twine-rs/commit/6df40279a5d6e5e5b1e9608d505bacb918accf66))
* re-export pkcs8 in twine_builder ([627cfb5](https://github.com/twine-protocol/twine-rs/commit/627cfb596d3a47088f704bf8dbe36e9c980b8ccf))
* remove "Registration" flow for http v2 api. batch saves. ([9164244](https://github.com/twine-protocol/twine-rs/commit/916424475a5e258685b77a743f6490b8134b55fb))
* rename Query to SingleQuery ([a9123f3](https://github.com/twine-protocol/twine-rs/commit/a9123f3bd3dadc4388e3536ffaaa33ba9a45ee59))
* rework resolver to return resolutions which validate query ([0231c67](https://github.com/twine-protocol/twine-rs/commit/0231c67e18e64d6697af4a7b59fd2c12a02fd954))
* Twine.strand and Twine.tixel return references ([dab681d](https://github.com/twine-protocol/twine-rs/commit/dab681d0fa3a00a34a280fc77955ba9f28b81b16))
* use Arc for sled store to prevent conflicts ([d81fa0a](https://github.com/twine-protocol/twine-rs/commit/d81fa0a97fdbf59d2557c226dc9fbd296bf1ceb0))


### Bug Fixes

* add async runtime feature flags ([7e6acba](https://github.com/twine-protocol/twine-rs/commit/7e6acbabd612f5f40b369fc004cad8e5ef58127b))
* batched range queries failing on ranges near batch size ([7be8f53](https://github.com/twine-protocol/twine-rs/commit/7be8f5314b7c9ad9b7028807e29cff5f44079d39))
* breaking change, remove timeout option ([166b59b](https://github.com/twine-protocol/twine-rs/commit/166b59b52dcb5d0e22cfcb6a832f8dc9ef1db0d3))
* breaking change. dag_json methods are now tagged_dag_json ([be828e5](https://github.com/twine-protocol/twine-rs/commit/be828e5ffb0cafccf6411ef24d681e6ef2a3b2f1))
* bug with query string parse ([0ea1adf](https://github.com/twine-protocol/twine-rs/commit/0ea1adf1e2b261b56a5d6b529d730d99ac492d30))
* bug with serde_dag_json and newtypes ([1f30ed1](https://github.com/twine-protocol/twine-rs/commit/1f30ed14867baebb31d9745c42a2f57664aa127b))
* builder payload default differed between first and next ([3fdaf9d](https://github.com/twine-protocol/twine-rs/commit/3fdaf9d84602272078efb681a765ca45a10af0c6))
* builder_v2 contained noop cross stitches method ([3df7105](https://github.com/twine-protocol/twine-rs/commit/3df71056b6ec0ef69546d0b4b6786f2a22b302b2))
* change language to unchecked_base so not imply unsafe rust ([c4a7841](https://github.com/twine-protocol/twine-rs/commit/c4a7841a30abdd75fce0cac387bd1808d737929e))
* change name from resolve_and_add to add_or_refresh ([23a5a7f](https://github.com/twine-protocol/twine-rs/commit/23a5a7f48d4d22573dce129a5db25544d4e9d2df))
* change payload extraction error type ([db6ffad](https://github.com/twine-protocol/twine-rs/commit/db6ffade8468c78163b36472a6211cbff772cb23))
* change twine builder signature to use const generics ([3b66b8b](https://github.com/twine-protocol/twine-rs/commit/3b66b8bf4e8e0b8592ee3e0df075b010937b5a12))
* **cli:** add info messages to check cmd ([c503416](https://github.com/twine-protocol/twine-rs/commit/c503416de5ce59bd9ac645266a0894efe05d9042))
* **cli:** change info message text ([0651335](https://github.com/twine-protocol/twine-rs/commit/0651335c94f786b8c4e458eee055d7f8f519539d))
* **cli:** permission change for keygen only in unix ([be72c5c](https://github.com/twine-protocol/twine-rs/commit/be72c5c7762c2c06357f84c3febcd749ebd7e0d2))
* cross_stitches of length 0 incorrectly validated ([f625e59](https://github.com/twine-protocol/twine-rs/commit/f625e59963014f19257d596a8e3ebec61f0dbcf4))
* cross-stitches weren't carrying through from previous ([2456bca](https://github.com/twine-protocol/twine-rs/commit/2456bca7a1092f02dd730dae78bee89fe5720eed))
* Encoded cross-stitches could have arbitrary order ([43e21e8](https://github.com/twine-protocol/twine-rs/commit/43e21e8dcbd522ff7f1bd22f1eb88af635adbb2f))
* flush car data periodically ([4f5be1d](https://github.com/twine-protocol/twine-rs/commit/4f5be1dfca12908581800b184aaecc8f589ab72f))
* flush pickle data on every save ([ee8d68e](https://github.com/twine-protocol/twine-rs/commit/ee8d68ee5cdb2414f13bd1c8ba3c99bfd2f9b92c))
* handle non-existant car file on startup ([c38bb75](https://github.com/twine-protocol/twine-rs/commit/c38bb7533cdd56aee0e88760682e9db8c06448bc))
* hash features in meta package ([b657d0b](https://github.com/twine-protocol/twine-rs/commit/b657d0ba496e9f42c39be69be52c4757e7e7cebc))
* include error response message from html body ([a80bdc4](https://github.com/twine-protocol/twine-rs/commit/a80bdc4521bda56e6c4d404fbe872810d9d3fed6))
* make biscuit dep optional ([ad02892](https://github.com/twine-protocol/twine-rs/commit/ad028924bda11702ce5739cd20c037fac7447b24))
* mysql store had incorrect range retrieval limits ([e22a80e](https://github.com/twine-protocol/twine-rs/commit/e22a80ee7bc3eaa191214f961b511b564496232b))
* non-exhaustive pattern match for sql store ([b0b4a8a](https://github.com/twine-protocol/twine-rs/commit/b0b4a8a8f820dd7f0b65eca357bde6d9478f7dd9))
* only save car on change ([060220f](https://github.com/twine-protocol/twine-rs/commit/060220f2913117302187a7229dd549b06c700a08))
* **pickledb_store:** incorrect handling of empty lists ([f42af8f](https://github.com/twine-protocol/twine-rs/commit/f42af8fd2c77c0838670a503c04d84341c0b7b83))
* reorganize and adjust BaseResolver to discourage client usage ([f470437](https://github.com/twine-protocol/twine-rs/commit/f4704378f333bfd42bf0a522f57b6b921fc531ad))
* RSA keys with different modulus sizes not correctly parsed ([3736c25](https://github.com/twine-protocol/twine-rs/commit/3736c25a4d11a79713dc0130e2aecaa9fc374b71))
* rsa usage wasn't behind rsa feature ([54fb398](https://github.com/twine-protocol/twine-rs/commit/54fb39878e3c970102defe7cd6c7abe2bba8d774))
* sled store key didn't account for smaller cid lengths ([885d896](https://github.com/twine-protocol/twine-rs/commit/885d8966831436c461dd9ced2a498b21e523bd18))
* sql store range resolve was broken ([28ee65b](https://github.com/twine-protocol/twine-rs/commit/28ee65bf0d5ff69f2813b7efd8e361af13352d7a))
* Tagged&lt;AnyTwine&gt; serialization incorrect ([a191587](https://github.com/twine-protocol/twine-rs/commit/a1915870564f986f9372c6d8605ff3ff85b12c16))
* tixel builder will copy specification from strand ([17848ee](https://github.com/twine-protocol/twine-rs/commit/17848ee920b70c5c0fdc9629496cc5bfd8d8a260))
* **twine_http_store:** batch stream saving ([a67a2f8](https://github.com/twine-protocol/twine-rs/commit/a67a2f8cea830a96fdb928410ff9ec102c788d19))
* use uppercase for key algorithm identifiers in v2 ([a9f16cf](https://github.com/twine-protocol/twine-rs/commit/a9f16cf0690a5b537b9485ceead796b9bfeadeeb))
