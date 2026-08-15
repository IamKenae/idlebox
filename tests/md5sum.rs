use std::fs;
use std::process::Command;

fn get_bin() -> String {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join("idlebox").to_str().unwrap().to_string()
}

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
    fs::write(&check_file, format!("5eb63bbbe01eeed093cb22bb8f5acdc3  {}\n", file.display())).unwrap();

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
    fs::write(&check_file, format!("5eb63bbbe01eeed093cb22bb8f5acdc3  {}\n", file.display())).unwrap();

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
