#[test]
fn ui() {
    let tests = trybuild::TestCases::new();

    tests.compile_fail("tests/ui/fail/*.rs");
    tests.pass("tests/ui/pass/*.rs");
}