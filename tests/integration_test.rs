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
    assert!(stdout.contains("echo"));
    assert!(stdout.contains("relax"));
}
