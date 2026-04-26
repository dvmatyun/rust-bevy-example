//! Smoke test — proves your toolchain works.
//!
//! Run: `cargo test --test 00_smoke`
//!
//! This test should already pass without any work from you. If it
//! doesn't, fix your `cargo` / `rustc` install before doing the
//! other test stages.

#[test]
fn one_plus_one_is_two() {
    assert_eq!(1 + 1, 2);
}

#[test]
fn library_links() {
    // Compiling this test exercises that `my_bevy_game` builds as a
    // library and can be linked from an integration test. We don't
    // actually need to call anything — the compile is the test.
    use my_bevy_game::*;
    let _: () = ();
}
