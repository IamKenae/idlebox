use std::fs;
use std::process::Command;

mod common;
use common::get_bin;
#[test]
fn test_b3sum_basic() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let output = Command::new(&bin).arg("b3sum").arg(&file).output().unwrap();

    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.starts_with("d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24  "));
}

#[test]
fn test_b3sum_check() {
    let bin = get_bin();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let check_file = dir.path().join("check.b3");
    fs::write(
        &check_file,
        format!(
            "d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24  {}\n",
            file.display()
        ),
    )
    .unwrap();

    let output = Command::new(&bin)
        .arg("b3sum")
        .arg("-c")
        .arg(&check_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains("OK"));
}

#[test]
fn test_b3sum_parallel_large() {
    use std::io::Write;
    let mut f = tempfile::NamedTempFile::new().unwrap();
    let data = vec![0x42u8; 1024 * 1024 + 10]; // 1MB + 10 bytes
    f.write_all(&data).unwrap();
    let path = f.path().to_str().unwrap().to_string();
    let bin = get_bin();
    let output = Command::new(&bin).arg("b3sum").arg(&path).output().unwrap();

    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains(&path));
    // Use hardcoded known good hash for this 1MB+10 byte vector to avoid external blake3 dependency
    let expected_hash = "4047d64869f6ac20b82026cc0ce75e2079a78a545586437c2856a9014b258e0b";

    let hash_part = out.split_whitespace().next().unwrap();
    assert_eq!(hash_part, expected_hash);
}
