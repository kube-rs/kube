/// Doctests for CI
pub mod docs {
    /// Root README.md
    pub mod readme {
        #![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../README.md"))]
    }
}
