use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn idlebox_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_idlebox"))
}

#[test]
fn test_ish_echo_command() {
    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg("echo hello")
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
}

#[test]
fn test_ish_true_command() {
    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg("true")
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
}

#[test]
fn test_ish_false_command() {
    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg("false")
        .output()
        .expect("failed to execute ish");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn test_ish_export_and_variable_expansion() {
    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg("export TEST=value; echo $TEST")
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "value");
}

#[test]
fn test_ish_and_operator() {
    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg("true && echo success")
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "success");
}

#[test]
fn test_ish_or_operator() {
    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg("false || echo fallback")
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "fallback");
}

#[test]
fn test_ish_semicolon_separator() {
    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg("echo first; echo second")
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("first"));
    assert!(stdout.contains("second"));
}

#[test]
fn test_ish_stdout_redirect() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_file = temp_dir.path().join("output.txt");

    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg(format!("echo hello > {}", output_file.display()))
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    let content = fs::read_to_string(&output_file).expect("failed to read output file");
    assert_eq!(content.trim(), "hello");
}

#[test]
fn test_ish_append_redirect() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_file = temp_dir.path().join("output.txt");

    fs::write(&output_file, "first\n").expect("failed to write initial content");

    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg(format!("echo second >> {}", output_file.display()))
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    let content = fs::read_to_string(&output_file).expect("failed to read output file");
    assert_eq!(content, "first\nsecond\n");
}

#[test]
fn test_ish_script_file() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let script_file = temp_dir.path().join("test.sh");

    fs::write(&script_file, "echo line1\necho line2\nexit 0\n").expect("failed to write script");

    let output = idlebox_bin()
        .arg("ish")
        .arg(script_file.to_str().unwrap())
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("line1"));
    assert!(stdout.contains("line2"));
}

#[test]
fn test_ish_script_with_exit_code() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let script_file = temp_dir.path().join("test.sh");

    fs::write(&script_file, "exit 42\n").expect("failed to write script");

    let output = idlebox_bin()
        .arg("ish")
        .arg(script_file.to_str().unwrap())
        .output()
        .expect("failed to execute ish");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(42));
}

#[test]
fn test_sh_alias() {
    let output = idlebox_bin()
        .arg("sh")
        .arg("-c")
        .arg("echo from sh")
        .output()
        .expect("failed to execute sh");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "from sh");
}

#[test]
fn test_ash_alias() {
    let output = idlebox_bin()
        .arg("ash")
        .arg("-c")
        .arg("echo from ash")
        .output()
        .expect("failed to execute ash");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "from ash");
}

#[test]
fn test_ish_for_loop() {
    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg("for i in 1 2 3; do echo $i; done")
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1"));
    assert!(stdout.contains("2"));
    assert!(stdout.contains("3"));
}

#[test]
fn test_ish_if_statement() {
    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg("if true; then echo yes; fi")
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "yes");
}

#[test]
fn test_ish_if_else_statement() {
    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg("if false; then echo yes; else echo no; fi")
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "no");
}

#[test]
fn test_ish_pipeline() {
    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg("echo hello | grep hello")
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
}

#[test]
fn test_ish_builtin_cd() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let dir_path = temp_dir.path();

    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg(format!("cd {}; pwd", dir_path.display()))
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(stdout.ends_with(dir_path.file_name().unwrap().to_str().unwrap()));
}

#[test]
fn test_ish_builtin_pwd() {
    let output = idlebox_bin()
        .arg("ish")
        .arg("-c")
        .arg("pwd")
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).is_empty());
}

#[test]
fn test_ish_complex_script() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let script_file = temp_dir.path().join("complex.sh");

    let script_content = r#"
export GREETING=Hello
export NAME=World
echo "$GREETING $NAME"
if true; then
    echo "Condition met"
fi
for i in a b c; do
    echo "Item: $i"
done
exit 0
"#;

    fs::write(&script_file, script_content).expect("failed to write script");

    let output = idlebox_bin()
        .arg("ish")
        .arg(script_file.to_str().unwrap())
        .output()
        .expect("failed to execute ish");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello World"));
    assert!(stdout.contains("Condition met"));
    assert!(stdout.contains("Item: a"));
    assert!(stdout.contains("Item: b"));
    assert!(stdout.contains("Item: c"));
}
