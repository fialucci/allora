use trybuild::TestCases;

#[test]
fn service_macro_ui() {
    let t = TestCases::new();
    // Passing cases (macro/pass)
    t.pass("tests/macro/pass/service_default_name.rs");
    t.pass("tests/macro/pass/service_explicit_name.rs");
    // Diagnostics (macro/diagnostics)
    t.compile_fail("tests/macro/diagnostics/service_fail_no_new.rs");
    t.compile_fail("tests/macro/diagnostics/service_fail_trait_impl.rs");
    t.compile_fail("tests/macro/diagnostics/service_fail_bad_name.rs");
    t.compile_fail("tests/macro/diagnostics/service_fail_generic_impl.rs");
}
