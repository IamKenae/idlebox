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
    assert!(stdout.contains("cp"));
    assert!(stdout.contains("echo"));
    assert!(stdout.contains("grep"));
    assert!(stdout.contains("head"));
    assert!(stdout.contains("ls"));
    assert!(stdout.contains("mkdir"));
    assert!(stdout.contains("mv"));
    assert!(stdout.contains("relax"));
    assert!(stdout.contains("rm"));
    assert!(stdout.contains("tail"));
    assert!(stdout.contains("touch"));
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

#[test]
fn test_install_creates_symlinks() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_install");
    let _ = fs::remove_dir_all(&tmp_dir);

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "--install", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Created symlink"));

    for applet in &["cat", "cp", "echo", "grep", "head", "ls", "mkdir", "mv", "relax", "rm", "tail", "touch"] {
        let link = tmp_dir.join(applet);
        assert!(link.exists(), "symlink for {} should exist", applet);
        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(meta.file_type().is_symlink(), "{} should be a symlink", applet);
    }

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_install_overwrites_existing() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_install_overwrite");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    fs::write(tmp_dir.join("echo"), "dummy").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "--install", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());

    let meta = fs::symlink_metadata(tmp_dir.join("echo")).unwrap();
    assert!(meta.file_type().is_symlink());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_install_creates_directory() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_install_newdir").join("sub");
    let _ = fs::remove_dir_all(tmp_dir.parent().unwrap());

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "--install", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(tmp_dir.exists());
    assert!(tmp_dir.join("echo").exists());

    let _ = fs::remove_dir_all(tmp_dir.parent().unwrap());
}

#[test]
fn test_install_symlink_invokes_applet() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_install_invoke");
    let _ = fs::remove_dir_all(&tmp_dir);

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "--install", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");
    assert!(output.status.success());

    let output = Command::new(tmp_dir.join("echo"))
        .args(["hello", "from", "symlink"])
        .output()
        .expect("failed to execute via symlink");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello from symlink");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_mkdir_basic() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_mkdir");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let target = tmp_dir.join("newdir");
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "mkdir", target.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(target.is_dir());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_mkdir_parents() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_mkdir_p");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let nested = tmp_dir.join("a").join("b").join("c");
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "mkdir", "-p", nested.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(nested.is_dir());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_mkdir_parents_no_error_existing() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_mkdir_p_exist");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "mkdir", "-p", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_mkdir_multiple() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_mkdir_multi");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let d1 = tmp_dir.join("dir1");
    let d2 = tmp_dir.join("dir2");
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "mkdir", d1.to_str().unwrap(), d2.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(d1.is_dir());
    assert!(d2.is_dir());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_mkdir_without_parents_fails_on_nested() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_mkdir_nop");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let nested = tmp_dir.join("x").join("y");
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "mkdir", nested.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_rm_file() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_rm");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("file.txt");
    fs::write(&file, "hello").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "rm", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(!file.exists());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_rm_rf() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_rm_rf");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let sub = tmp_dir.join("subdir");
    fs::create_dir_all(sub.join("nested")).unwrap();
    fs::write(sub.join("file1.txt"), "content1").unwrap();
    fs::write(sub.join("nested").join("file2.txt"), "content2").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "rm", "-rf", sub.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(!sub.exists());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_rm_force_nonexistent() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "rm", "-f", "/tmp/idlebox_nonexistent_file_xyz"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
}

#[test]
fn test_rm_without_recursive_fails_on_dir() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_rm_norec");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let sub = tmp_dir.join("subdir");
    fs::create_dir(&sub).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "rm", sub.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    assert!(sub.exists());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_cp_file() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cp");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let src = tmp_dir.join("source.txt");
    let dst = tmp_dir.join("dest.txt");
    fs::write(&src, "copy me").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "cp", src.to_str().unwrap(), dst.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(dst.exists());
    assert_eq!(fs::read_to_string(&dst).unwrap(), "copy me");
    assert!(src.exists());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_cp_recursive() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cp_r");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let src_dir = tmp_dir.join("src");
    fs::create_dir_all(src_dir.join("sub")).unwrap();
    fs::write(src_dir.join("file1.txt"), "one").unwrap();
    fs::write(src_dir.join("sub").join("file2.txt"), "two").unwrap();

    let dst_dir = tmp_dir.join("dst");
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "cp", "-r", src_dir.to_str().unwrap(), dst_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(dst_dir.join("file1.txt").exists());
    assert!(dst_dir.join("sub").join("file2.txt").exists());
    assert_eq!(fs::read_to_string(dst_dir.join("file1.txt")).unwrap(), "one");
    assert_eq!(fs::read_to_string(dst_dir.join("sub").join("file2.txt")).unwrap(), "two");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_cp_multiple_to_dir() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cp_multi");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let f1 = tmp_dir.join("f1.txt");
    let f2 = tmp_dir.join("f2.txt");
    let dest = tmp_dir.join("dest");
    fs::write(&f1, "one").unwrap();
    fs::write(&f2, "two").unwrap();
    fs::create_dir(&dest).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "cp", f1.to_str().unwrap(), f2.to_str().unwrap(), dest.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(dest.join("f1.txt").exists());
    assert!(dest.join("f2.txt").exists());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_mv_rename_file() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_mv_rename");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let src = tmp_dir.join("old.txt");
    let dst = tmp_dir.join("new.txt");
    fs::write(&src, "rename me").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "mv", src.to_str().unwrap(), dst.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(!src.exists());
    assert!(dst.exists());
    assert_eq!(fs::read_to_string(&dst).unwrap(), "rename me");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_mv_multiple_to_dir() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_mv_multi");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let f1 = tmp_dir.join("f1.txt");
    let f2 = tmp_dir.join("f2.txt");
    let dest = tmp_dir.join("dest");
    fs::write(&f1, "one").unwrap();
    fs::write(&f2, "two").unwrap();
    fs::create_dir(&dest).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "mv", f1.to_str().unwrap(), f2.to_str().unwrap(), dest.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(!f1.exists());
    assert!(!f2.exists());
    assert!(dest.join("f1.txt").exists());
    assert!(dest.join("f2.txt").exists());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_mv_directory() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_mv_dir");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let src = tmp_dir.join("srcdir");
    let dst = tmp_dir.join("dstdir");
    fs::create_dir_all(src.join("nested")).unwrap();
    fs::write(src.join("nested").join("file.txt"), "data").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "mv", src.to_str().unwrap(), dst.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(!src.exists());
    assert!(dst.join("nested").join("file.txt").exists());
    assert_eq!(fs::read_to_string(dst.join("nested").join("file.txt")).unwrap(), "data");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_touch_create_file() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_touch");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("newfile.txt");
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "touch", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(file.exists());
    assert_eq!(fs::read_to_string(&file).unwrap(), "");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_touch_multiple_files() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_touch_multi");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let f1 = tmp_dir.join("a.txt");
    let f2 = tmp_dir.join("b.txt");
    let f3 = tmp_dir.join("c.txt");
    let output = Command::new("cargo")
        .args([
            "run", "--quiet", "--", "touch",
            f1.to_str().unwrap(),
            f2.to_str().unwrap(),
            f3.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(f1.exists());
    assert!(f2.exists());
    assert!(f3.exists());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_touch_updates_existing_file() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_touch_update");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("existing.txt");
    fs::write(&file, "content").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "touch", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(fs::read_to_string(&file).unwrap(), "content");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_head_default_lines() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_head");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    let lines: Vec<String> = (1..=20).map(|i| format!("line {}", i)).collect();
    fs::write(&file, lines.join("\n")).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "head", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out_lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(out_lines.len(), 10);
    assert_eq!(out_lines[0], "line 1");
    assert_eq!(out_lines[9], "line 10");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_head_n_lines() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_head_n");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    let lines: Vec<String> = (1..=20).map(|i| format!("line {}", i)).collect();
    fs::write(&file, lines.join("\n")).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "head", "-n", "5", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out_lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(out_lines.len(), 5);
    assert_eq!(out_lines[0], "line 1");
    assert_eq!(out_lines[4], "line 5");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_head_bytes() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_head_c");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "Hello, World! This is a test.").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "head", "-c", "5", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_head_stdin() {
    let mut child = Command::new("cargo")
        .args(["run", "--quiet", "--", "head", "-n", "3"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn process");

    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin.write_all(b"line1\nline2\nline3\nline4\nline5\n").unwrap();
    }

    let output = child.wait_with_output().expect("failed to wait");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out_lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(out_lines.len(), 3);
    assert_eq!(out_lines[0], "line1");
}

#[test]
fn test_tail_default_lines() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tail");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    let lines: Vec<String> = (1..=20).map(|i| format!("line {}", i)).collect();
    fs::write(&file, lines.join("\n")).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "tail", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out_lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(out_lines.len(), 10);
    assert_eq!(out_lines[0], "line 11");
    assert_eq!(out_lines[9], "line 20");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tail_n_lines() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tail_n");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    let lines: Vec<String> = (1..=20).map(|i| format!("line {}", i)).collect();
    fs::write(&file, lines.join("\n")).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "tail", "-n", "3", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out_lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(out_lines.len(), 3);
    assert_eq!(out_lines[0], "line 18");
    assert_eq!(out_lines[2], "line 20");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tail_stdin() {
    let mut child = Command::new("cargo")
        .args(["run", "--quiet", "--", "tail", "-n", "2"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn process");

    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin.write_all(b"line1\nline2\nline3\nline4\nline5\n").unwrap();
    }

    let output = child.wait_with_output().expect("failed to wait");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out_lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(out_lines.len(), 2);
    assert_eq!(out_lines[0], "line4");
    assert_eq!(out_lines[1], "line5");
}

#[test]
fn test_tail_bytes() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tail_c");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "Hello, World!").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "tail", "-c", "6", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "World!");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_grep_basic() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_grep");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "apple\nbanana\napple pie\ncherry\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "grep", "apple", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out_lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(out_lines.len(), 2);
    assert_eq!(out_lines[0], "apple");
    assert_eq!(out_lines[1], "apple pie");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_grep_ignore_case() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_grep_i");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "Error\nerror\nERROR\nwarning\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "grep", "-i", "error", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out_lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(out_lines.len(), 3);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_grep_line_number() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_grep_n");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "alpha\nbeta\ngamma\ndelta\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "grep", "-n", "gamma", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "3:gamma");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_grep_invert_match() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_grep_v");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "apple\nbanana\napple pie\ncherry\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "grep", "-v", "apple", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out_lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(out_lines.len(), 2);
    assert_eq!(out_lines[0], "banana");
    assert_eq!(out_lines[1], "cherry");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_grep_count() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_grep_c");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "apple\nbanana\napple pie\ncherry\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "grep", "-c", "apple", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "2");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_grep_stdin() {
    let mut child = Command::new("cargo")
        .args(["run", "--quiet", "--", "grep", "-i", "error"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn process");

    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin.write_all(b"Info: ok\nError: fail\nWarning: maybe\nerror: again\n").unwrap();
    }

    let output = child.wait_with_output().expect("failed to wait");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out_lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(out_lines.len(), 2);
    assert_eq!(out_lines[0], "Error: fail");
    assert_eq!(out_lines[1], "error: again");
}

#[test]
fn test_grep_ignore_case_with_line_number() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_grep_in");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "Error here\nno match\nERROR there\nerror again\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "grep", "-in", "error", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out_lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(out_lines.len(), 3);
    assert_eq!(out_lines[0], "1:Error here");
    assert_eq!(out_lines[1], "3:ERROR there");
    assert_eq!(out_lines[2], "4:error again");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_grep_no_match_returns_1() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_grep_nomatch");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "hello\nworld\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "grep", "zzz", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert_eq!(output.status.code(), Some(1));

    let _ = fs::remove_dir_all(&tmp_dir);
}
