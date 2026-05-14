#[test]
fn macro_compile_contracts() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/macros/pass/*.rs");
    tests.compile_fail("tests/ui/macros/fail/*.rs");
}
