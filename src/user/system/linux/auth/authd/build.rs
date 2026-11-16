//! A sabotaged build (`src/sabotage.rs`) is one compiled with
//! `XAQU_AUTH_SABOTAGE` set and not empty. Such a build, and only such a
//! build, gets `cfg(xaqu_auth_sabotage)`, which puts the marker xtask looks
//! for into the program, so that xtask can refuse to package it anywhere but
//! in `test-auth --sabotage`.

fn main() {
    println!("cargo:rerun-if-env-changed=XAQU_AUTH_SABOTAGE");
    println!("cargo:rustc-check-cfg=cfg(xaqu_auth_sabotage)");
    if std::env::var("XAQU_AUTH_SABOTAGE").is_ok_and(|name| !name.is_empty()) {
        println!("cargo:rustc-cfg=xaqu_auth_sabotage");
    }
}
