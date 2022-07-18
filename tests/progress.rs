#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/01-parse-fn.rs");
    t.pass("tests/actix-sqlx.rs");
}
