#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/parse-just-fn.rs");
    t.pass("tests/actix-sqlx.rs");
}
