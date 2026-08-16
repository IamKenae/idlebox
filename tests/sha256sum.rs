use std::fs;
use std::process::Command;

mod common;
use common::get_bin;

#[test]
fn test_sha256sum_basic() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let output = Command::new(&bin)
        .arg("sha256sum")
        .arg(&file)
        .output()
        .unwrap();

    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    // echo -n "hello world" | sha256sum -> b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
    assert!(out.starts_with("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9  "));
}

#[test]
fn test_sha256sum_check() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let check_file = dir.path().join("check.sha256");
    fs::write(
        &check_file,
        format!(
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9  {}\n",
            file.display()
        ),
    )
    .unwrap();

    let output = Command::new(&bin)
        .arg("sha256sum")
        .arg("-c")
        .arg(&check_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains("OK"));
}

#[test]
fn test_sha256sum_empty_input() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("empty.txt");
    std::fs::write(&file, "").unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha256sum")
        .arg(&file)
        .output()
        .unwrap();
    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.starts_with("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  "));
}

#[test]
fn test_sha256sum_binary_mode() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.bin");
    std::fs::write(&file, "hello world").unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha256sum")
        .arg("-b")
        .arg(&file)
        .output()
        .unwrap();
    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9 *"));
}

#[test]
fn test_sha256sum_nonexistent_file() {
    let bin = get_bin();
    let output = std::process::Command::new(&bin)
        .arg("sha256sum")
        .arg("does_not_exist.txt")
        .output()
        .unwrap();
    assert!(!output.status.success());
}
