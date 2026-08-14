use std::fs;
use std::io::Write;
use std::process::Command;

#[test]
fn test_echo_basic() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "echo", "hello", "world"])
        .output()
        .expect("failed to execute process");
    
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello world");
}

#[test]
fn test_echo_no_newline() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "echo", "-n", "test"])
        .output()
        .expect("failed to execute process");
    
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "test");
}

#[test]
fn test_unknown_applet() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "nonexistent"])
        .output()
        .expect("failed to execute process");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("applet not found"));
}

#[test]
fn test_list_applets() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "list"])
        .output()
        .expect("failed to execute process");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cat"));
    assert!(stdout.contains("echo"));
    assert!(stdout.contains("ls"));
    assert!(stdout.contains("relax"));
}

#[test]
fn test_help_short_flag() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "relax", "-h"])
        .output()
        .expect("failed to execute process");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("relax"));
}

#[test]
fn test_help_long_flag() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "echo", "--help"])
        .output()
        .expect("failed to execute process");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
}

#[test]
fn test_cat_file() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cat");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    
    let test_file = tmp_dir.join("test.txt");
    let mut f = fs::File::create(&test_file).unwrap();
    writeln!(f, "line one").unwrap();
    writeln!(f, "line two").unwrap();
    writeln!(f, "line three").unwrap();
    
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "cat", test_file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("line one"));
    assert!(stdout.contains("line two"));
    assert!(stdout.contains("line three"));
    
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_cat_number_lines() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cat_n");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    
    let test_file = tmp_dir.join("test.txt");
    let mut f = fs::File::create(&test_file).unwrap();
    writeln!(f, "first").unwrap();
    writeln!(f, "second").unwrap();
    
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "cat", "-n", test_file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1"));
    assert!(stdout.contains("2"));
    assert!(stdout.contains("first"));
    assert!(stdout.contains("second"));
    
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_cat_stdin() {
    let mut child = Command::new("cargo")
        .args(["run", "--quiet", "--", "cat"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn process");
    
    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin.write_all(b"hello from stdin\n").unwrap();
    }
    
    let output = child.wait_with_output().expect("failed to wait");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("hello from stdin"));
}

#[test]
fn test_ls_basic() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_ls");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    
    fs::File::create(tmp_dir.join("file1.txt")).unwrap();
    fs::File::create(tmp_dir.join("file2.txt")).unwrap();
    fs::create_dir(tmp_dir.join("subdir")).unwrap();
    
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "ls", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
    assert!(stdout.contains("subdir"));
    
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_ls_long_format() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_ls_l");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    
    fs::File::create(tmp_dir.join("testfile.txt")).unwrap();
    
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "ls", "-l", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("testfile.txt"));
    assert!(stdout.contains("-rw"));
    
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_ls_all_flag() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_ls_a");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    
    fs::File::create(tmp_dir.join(".hidden")).unwrap();
    fs::File::create(tmp_dir.join("visible")).unwrap();
    
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "ls", "-a", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(".hidden"));
    assert!(stdout.contains("visible"));
    
    let _ = fs::remove_dir_all(&tmp_dir);
}
