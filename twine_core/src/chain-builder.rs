use crate::twine::{Chain, Mixin, DEFAULT_SPECIFICATION, TwineError};
use crate::sign::Signer;
use libipld::cid::multihash;

pub struct ChainBuilder {
    source: String,
    specification: String,
    radix: u32,
    mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
    meta: Option<Ipld>, // TODO: should be a map?
    signer: Signer,
    hasher: multihash::Code
}

impl BuildChain {
    pub fn new(source: String, signer: Signer, hasher: multihash::Code) -> Self {
        Self {
            source,
            specification: DEFAULT_SPECIFICATION.to_string(),
            radix: 32,
            mixins: Vec::new(),
            meta: None,
            signer,
            hasher   
        }
    }
    pub fn source(mut self, source: String) -> Self {
        self.source = source;
        self

    };
    fn specification(mut self, specification: String) -> Self {
        self.specification = specification;
        self
    };
    fn set_engine(mut self, radix: u32) -> Self {
        self.radix = radix;
        self
    }
    fn mixin(mut self, mixin: Mixin) -> Self {
        self.mixins.push(mixin);
        self
    }
    fn mixins(mut self, mixins: Mixin) -> Self {
        self.mixins.push_all(mixin);
        self
    }
    fn meta(mut self, meta: Ipld) -> Self {
        self.meta = Some(meta);
        self
    }
    fn signer(mut self, signer: Signer) -> Self {
        self.signer = signer;
        self
    }
    fn hasher(mut self, hasher: multihash::Code) -> Self {
        self.hasher = hasher;
        self
    }
    fn build(self) -> Result<Chain, TwineError> {
        let chain = Chain::build_chain( // TODO: do not use build_chain here
            ChainContent {
                source: self.source,
                specification: self.specification,
                radix: self.radix,
                key: self.signer.key,
                mixins: self.mixins,
                meta: self.meta
            },
            self.signer,
            self.hasher,
        );
    }
}

