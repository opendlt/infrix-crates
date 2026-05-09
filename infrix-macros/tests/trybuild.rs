#[test]
fn shape_macros_reject_malformed_metadata() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild/*.rs");
}
