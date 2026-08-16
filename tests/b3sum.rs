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
    let data = vec![0x42u8; 3 * 1024 * 1024 + 10]; // 3MB + 10 bytes
    f.write_all(&data).unwrap();
    let path = f.path().to_str().unwrap().to_string();
    let bin = get_bin();
    let output = Command::new(&bin).arg("b3sum").arg(&path).output().unwrap();

    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains(&path));
    // Use hardcoded known good hash for this 3MB+10 byte vector to avoid external blake3 dependency.
    // Verified against the standard blake3 crate.
    let expected_hash = "82551d84716bd712464a55d26663b0a4f94fdaf30595b5313507d3b445665a1a";

    let hash_part = out.split_whitespace().next().unwrap();
    assert_eq!(hash_part, expected_hash);
}

#[test]
fn test_b3sum_parallel_large_remainder() {
    use std::io::Write;
    let mut f = tempfile::NamedTempFile::new().unwrap();
    let data = vec![0x42u8; 3 * 1024 * 1024 + 500 * 1024]; // 3MB + 500KB
    f.write_all(&data).unwrap();
    let path = f.path().to_str().unwrap().to_string();
    let bin = get_bin();
    let output = std::process::Command::new(&bin)
        .arg("b3sum")
        .arg(&path)
        .output()
        .unwrap();

    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains(&path));
    // Verified against the standard blake3 crate.
    let expected_hash = "977b124db3be924c7b01a2790a966cd4d46a949f8a03de5848f8439043f540f9";

    let hash_part = out.split_whitespace().next().unwrap();
    assert_eq!(hash_part, expected_hash);
}
