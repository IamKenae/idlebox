use std::fs;
use std::process::Command;

mod common;
use common::get_bin;

#[test]
fn test_md5sum_basic() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let output = Command::new(&bin)
        .arg("md5sum")
        .arg(&file)
        .output()
        .unwrap();

    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.starts_with("5eb63bbbe01eeed093cb22bb8f5acdc3  "));
}

#[test]
fn test_md5sum_check() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let check_file = dir.path().join("check.md5");
    fs::write(
        &check_file,
        format!("5eb63bbbe01eeed093cb22bb8f5acdc3  {}\n", file.display()),
    )
    .unwrap();

    let output = Command::new(&bin)
        .arg("md5sum")
        .arg("-c")
        .arg(&check_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains("OK"));
}

#[test]
fn test_md5sum_check_status() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let check_file = dir.path().join("check.md5");
    fs::write(
        &check_file,
        format!("5eb63bbbe01eeed093cb22bb8f5acdc3  {}\n", file.display()),
    )
    .unwrap();

    let output = Command::new(&bin)
        .arg("md5sum")
        .arg("-c")
        .arg("--status")
        .arg(&check_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn test_md5sum_empty_input() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("empty.txt");
    fs::write(&file, "").unwrap();

    let output = Command::new(&bin)
        .arg("md5sum")
        .arg(&file)
        .output()
        .unwrap();
    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.starts_with("d41d8cd98f00b204e9800998ecf8427e  "));
}

#[test]
fn test_md5sum_stdin() {
    let bin = get_bin();
    use std::io::Write;
    let mut child = Command::new(&bin)
        .arg("md5sum")
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
    assert!(out.starts_with("5eb63bbbe01eeed093cb22bb8f5acdc3  -"));
}

#[test]
fn test_md5sum_binary_mode() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.bin");
    fs::write(&file, "hello world").unwrap();

    let output = Command::new(&bin)
        .arg("md5sum")
        .arg("-b")
        .arg(&file)
        .output()
        .unwrap();
    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    // Binary mode output has a '*' before the file name
    assert!(out.contains("5eb63bbbe01eeed093cb22bb8f5acdc3 *"));
}

#[test]
fn test_md5sum_invalid_options() {
    let bin = get_bin();
    let output = Command::new(&bin)
        .arg("md5sum")
        .arg("--foo")
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_md5sum_check_wrong_hash() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let check_file = dir.path().join("check.md5");
    // Incorrect hash
    fs::write(
        &check_file,
        format!("00000000000000000000000000000000  {}\n", file.display()),
    )
    .unwrap();

    let output = Command::new(&bin)
        .arg("md5sum")
        .arg("-c")
        .arg(&check_file)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains("FAILED"));

    // Test --status with wrong hash
    let status_output = Command::new(&bin)
        .arg("md5sum")
        .arg("-c")
        .arg("--status")
        .arg(&check_file)
        .output()
        .unwrap();
    assert!(!status_output.status.success());
    assert!(status_output.stdout.is_empty());
}

#[test]
fn test_md5sum_nonexistent_file() {
    let bin = get_bin();
    let output = Command::new(&bin)
        .arg("md5sum")
        .arg("does_not_exist.txt")
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_md5sum_multiple_files() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file1 = dir.path().join("1.txt");
    let file2 = dir.path().join("2.txt");
    fs::write(&file1, "hello world").unwrap();
    fs::write(&file2, "").unwrap();

    let output = Command::new(&bin)
        .arg("md5sum")
        .arg(&file1)
        .arg(&file2)
        .output()
        .unwrap();
    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("5eb63bbbe01eeed093cb22bb8f5acdc3  "));
    assert!(lines[1].starts_with("d41d8cd98f00b204e9800998ecf8427e  "));
}

#[test]
fn test_md5sum_end_of_options() {
    let bin = get_bin();
    use std::io::Write;
    let mut child = std::process::Command::new(&bin)
        .arg("md5sum")
        .arg("--")
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
    assert!(out.starts_with("5eb63bbbe01eeed093cb22bb8f5acdc3  -"));
}
