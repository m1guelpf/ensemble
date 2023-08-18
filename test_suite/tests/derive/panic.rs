use trybuild::TestCases;

#[test]
fn ensure_snapshot_panics() {
    let t = TestCases::new();
    t.compile_fail("tests/derive/panic/*.rs");
}
