// Test that `kube-derive` outputs helpful error messages.
// If you make a change, remove `tests/ui/*.stderr` and run `cargo test`.
// Then copy the files that appear under `wip/` if it's what you expected.
// Alternatively, run `TRYBUILD=overwrite cargo test`.
// See https://github.com/dtolnay/trybuild
#[test]
fn test_failures() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
