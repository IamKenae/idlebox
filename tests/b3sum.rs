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
    let output = Command::new(&bin)
        .arg("b3sum")
        .arg(&path)
        .output()
        .unwrap();

    assert!(output.status.success());
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains(&path));
    // The hash should be 64 characters long
    let hash_part = out.split_whitespace().next().unwrap();
    assert_eq!(hash_part.len(), 64);
}
