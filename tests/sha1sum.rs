use std::fs;
use std::process::Command;

mod common;
use common::get_bin;

#[test]
fn test_sha1sum_basic() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let output = Command::new(&bin)
        .arg("sha1sum")
        .arg(&file)
        .output()
        .unwrap();

    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    // echo -n "hello world" | sha1sum -> 2aae6c35c94fcfb415dbe95f408b9ce91ee846ed
    assert!(out.starts_with("2aae6c35c94fcfb415dbe95f408b9ce91ee846ed  "));
}

#[test]
fn test_sha1sum_check() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let check_file = dir.path().join("check.sha1");
    fs::write(
        &check_file,
        format!(
            "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed  {}\n",
            file.display()
        ),
    )
    .unwrap();

    let output = Command::new(&bin)
        .arg("sha1sum")
        .arg("-c")
        .arg(&check_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains("OK"));
}

#[test]
fn test_sha1sum_empty_input() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("empty.txt");
    std::fs::write(&file, "").unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha1sum")
        .arg(&file)
        .output()
        .unwrap();
    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.starts_with("da39a3ee5e6b4b0d3255bfef95601890afd80709  "));
}

#[test]
fn test_sha1sum_binary_mode() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.bin");
    std::fs::write(&file, "hello world").unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha1sum")
        .arg("-b")
        .arg(&file)
        .output()
        .unwrap();
    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains("2aae6c35c94fcfb415dbe95f408b9ce91ee846ed *"));
}

#[test]
fn test_sha1sum_nonexistent_file() {
    let bin = get_bin();
    let output = std::process::Command::new(&bin)
        .arg("sha1sum")
        .arg("does_not_exist.txt")
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_sha1sum_stdin() {
    let bin = get_bin();
    use std::io::Write;
    let mut child = std::process::Command::new(&bin)
        .arg("sha1sum")
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
    assert!(out.starts_with("2aae6c35c94fcfb415dbe95f408b9ce91ee846ed  -"));
}

#[test]
fn test_sha1sum_check_wrong_hash() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    std::fs::write(&file, "hello world").unwrap();

    let check_file = dir.path().join("check.sha1");
    std::fs::write(
        &check_file,
        format!(
            "0000000000000000000000000000000000000000  {}\n",
            file.display()
        ),
    )
    .unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha1sum")
        .arg("-c")
        .arg(&check_file)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains("FAILED"));
}

#[test]
fn test_sha1sum_multiple_files() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file1 = dir.path().join("1.txt");
    let file2 = dir.path().join("2.txt");
    std::fs::write(&file1, "hello world").unwrap();
    std::fs::write(&file2, "").unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha1sum")
        .arg(&file1)
        .arg(&file2)
        .output()
        .unwrap();
    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("2aae6c35c94fcfb415dbe95f408b9ce91ee846ed  "));
    assert!(lines[1].starts_with("da39a3ee5e6b4b0d3255bfef95601890afd80709  "));
}

#[test]
fn test_sha1sum_check_status_wrong_hash() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    std::fs::write(&file, "hello world").unwrap();

    let check_file = dir.path().join("check.sha1");
    std::fs::write(
        &check_file,
        format!(
            "0000000000000000000000000000000000000000  {}\n",
            file.display()
        ),
    )
    .unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha1sum")
        .arg("-c")
        .arg("--status")
        .arg(&check_file)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn test_sha1sum_check_malformed() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let check_file = dir.path().join("check.sha1");
    std::fs::write(&check_file, "this is a malformed line\n").unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha1sum")
        .arg("-c")
        .arg(&check_file)
        .output()
        .unwrap();
    // GNU coreutils behavior: Malformed lines exit 0, but produce a warning on stderr
    assert!(output.status.success());
    let err = String::from_utf8(output.stderr).unwrap();
    assert!(err.contains("WARNING: 1 lines are improperly formatted"));
}
