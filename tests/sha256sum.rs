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

#[test]
fn test_sha256sum_stdin() {
    let bin = get_bin();
    use std::io::Write;
    let mut child = std::process::Command::new(&bin)
        .arg("sha256sum")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"hello world")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.starts_with("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9  -"));
}

#[test]
fn test_sha256sum_check_wrong_hash() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    std::fs::write(&file, "hello world").unwrap();

    let check_file = dir.path().join("check.sha256");
    std::fs::write(
        &check_file,
        format!(
            "0000000000000000000000000000000000000000000000000000000000000000  {}\n",
            file.display()
        ),
    )
    .unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha256sum")
        .arg("-c")
        .arg(&check_file)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains("FAILED"));
}

#[test]
fn test_sha256sum_multiple_files() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file1 = dir.path().join("1.txt");
    let file2 = dir.path().join("2.txt");
    std::fs::write(&file1, "hello world").unwrap();
    std::fs::write(&file2, "").unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha256sum")
        .arg(&file1)
        .arg(&file2)
        .output()
        .unwrap();
    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(
        lines[0].starts_with("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9  ")
    );
    assert!(
        lines[1].starts_with("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  ")
    );
}

#[test]
fn test_sha256sum_check_status_wrong_hash() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    std::fs::write(&file, "hello world").unwrap();

    let check_file = dir.path().join("check.sha256");
    std::fs::write(
        &check_file,
        format!(
            "0000000000000000000000000000000000000000000000000000000000000000  {}\n",
            file.display()
        ),
    )
    .unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha256sum")
        .arg("-c")
        .arg("--status")
        .arg(&check_file)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn test_sha256sum_check_malformed() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let check_file = dir.path().join("check.sha256");
    std::fs::write(&check_file, "this is a malformed line\n").unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha256sum")
        .arg("-c")
        .arg(&check_file)
        .output()
        .unwrap();
    assert!(output.status.success());
    let err = String::from_utf8(output.stderr).unwrap();
    assert!(err.contains("WARNING: 1 lines are improperly formatted"));
}
