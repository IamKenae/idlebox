use std::fs;
use std::process::Command;

mod common;
use common::get_bin;

#[test]
fn test_sha512sum_basic() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let output = Command::new(&bin)
        .arg("sha512sum")
        .arg(&file)
        .output()
        .unwrap();

    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    // echo -n "hello world" | sha512sum -> 309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f
    assert!(out.starts_with("309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f  "));
}

#[test]
fn test_sha512sum_check() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let check_file = dir.path().join("check.sha512");
    fs::write(&check_file, format!("309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f  {}\n", file.display())).unwrap();

    let output = Command::new(&bin)
        .arg("sha512sum")
        .arg("-c")
        .arg(&check_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains("OK"));
}

#[test]
fn test_sha512sum_empty_input() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("empty.txt");
    std::fs::write(&file, "").unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha512sum")
        .arg(&file)
        .output()
        .unwrap();
    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.starts_with("cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e  "));
}

#[test]
fn test_sha512sum_binary_mode() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.bin");
    std::fs::write(&file, "hello world").unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha512sum")
        .arg("-b")
        .arg(&file)
        .output()
        .unwrap();
    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains("309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f *"));
}

#[test]
fn test_sha512sum_nonexistent_file() {
    let bin = get_bin();
    let output = std::process::Command::new(&bin)
        .arg("sha512sum")
        .arg("does_not_exist.txt")
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_sha512sum_stdin() {
    let bin = get_bin();
    use std::io::Write;
    let mut child = std::process::Command::new(&bin)
        .arg("sha512sum")
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
    assert!(out.starts_with("309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f  -"));
}

#[test]
fn test_sha512sum_check_wrong_hash() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    std::fs::write(&file, "hello world").unwrap();

    let check_file = dir.path().join("check.sha512");
    std::fs::write(
        &check_file,
        format!("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000  {}\n", file.display()),
    ).unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha512sum")
        .arg("-c")
        .arg(&check_file)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains("FAILED"));
}

#[test]
fn test_sha512sum_multiple_files() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file1 = dir.path().join("1.txt");
    let file2 = dir.path().join("2.txt");
    std::fs::write(&file1, "hello world").unwrap();
    std::fs::write(&file2, "").unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha512sum")
        .arg(&file1)
        .arg(&file2)
        .output()
        .unwrap();
    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f  "));
    assert!(lines[1].starts_with("cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e  "));
}

#[test]
fn test_sha512sum_check_status_wrong_hash() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    std::fs::write(&file, "hello world").unwrap();

    let check_file = dir.path().join("check.sha512");
    std::fs::write(
        &check_file,
        format!("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000  {}\n", file.display()),
    ).unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha512sum")
        .arg("-c")
        .arg("--status")
        .arg(&check_file)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn test_sha512sum_check_malformed() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let check_file = dir.path().join("check.sha512");
    std::fs::write(&check_file, "this is a malformed line\n").unwrap();

    let output = std::process::Command::new(&bin)
        .arg("sha512sum")
        .arg("-c")
        .arg(&check_file)
        .output()
        .unwrap();
    assert!(output.status.success());
    let err = String::from_utf8(output.stderr).unwrap();
    assert!(err.contains("WARNING: 1 lines are improperly formatted"));
}
