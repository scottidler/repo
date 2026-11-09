//! `repo --version` reports the anchored build version computed by `build.rs`.

use std::process::Command;

#[test]
fn version_flag_prints_build_version() {
    let out = Command::new(env!("CARGO_BIN_EXE_repo"))
        .arg("--version")
        .output()
        .expect("run repo --version");
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).expect("utf8");
    assert_eq!(stdout.trim(), format!("repo {}", env!("BUILD_VERSION")));
}

#[test]
fn build_version_is_anchored_on_cargo_toml_version() {
    let root = format!("v{}", env!("CARGO_PKG_VERSION"));
    let built = env!("BUILD_VERSION");
    assert!(
        built == root || built.starts_with(&format!("{root}-")),
        "{built} is not anchored on {root}"
    );
}
