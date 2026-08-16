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
