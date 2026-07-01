#[test]
fn typed_input_handles_do_not_cross_domains() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/input_type_mismatch.rs");
}
