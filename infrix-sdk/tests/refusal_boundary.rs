#[test]
fn direct_governance_refusal_markers_cannot_be_implemented_by_a_contract() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/trybuild/direct_governance_marker.rs");
}
