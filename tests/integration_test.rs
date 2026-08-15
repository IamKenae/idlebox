use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

fn installed_applet_path(directory: &Path, applet: &str) -> PathBuf {
    directory.join(format!("{}{}", applet, std::env::consts::EXE_SUFFIX))
}

fn install_test_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("idlebox_test_{}_{}", name, std::process::id()))
}

fn run_install(directory: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_idlebox"))
        .args(["--install", directory.to_str().unwrap()])
        .output()
        .expect("failed to execute idlebox --install")
}

fn assert_command_success(output: &std::process::Output, operation: &str) {
    assert!(
        output.status.success(),
        "{} failed with {}\nstdout:\n{}\nstderr:\n{}",
        operation,
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

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
    assert!(stdout.contains("chgrp"));
    assert!(stdout.contains("chmod"));
    assert!(stdout.contains("chown"));
    assert!(stdout.contains("cp"));
    assert!(stdout.contains("cut"));
    assert!(stdout.contains("df"));
    assert!(stdout.contains("du"));
    assert!(stdout.contains("echo"));
    assert!(stdout.contains("expr"));
    assert!(stdout.contains("find"));
    assert!(stdout.contains("free"));
    assert!(stdout.contains("grep"));
    assert!(stdout.contains("head"));
    assert!(stdout.contains("id"));
    assert!(stdout.contains("kill"));
    assert!(stdout.contains("ln"));
    assert!(stdout.contains("ls"));
    assert!(stdout.contains("mkdir"));
    assert!(stdout.contains("mv"));
    assert!(stdout.contains("ps"));
    assert!(stdout.contains("readlink"));
    assert!(stdout.contains("relax"));
    assert!(stdout.contains("rm"));
    assert!(stdout.contains("sort"));
    assert!(stdout.contains("su"));
    assert!(stdout.contains("tail"));
    assert!(stdout.contains("test"));
    assert!(stdout.contains("touch"));
    assert!(stdout.contains("tr"));
    assert!(stdout.contains("uname"));
    assert!(stdout.contains("uniq"));
    assert!(stdout.contains("uptime"));
    assert!(stdout.contains("wc"));
    assert!(stdout.contains("whoami"));
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
fn test_install_creates_launchers() {
    let tmp_dir = install_test_dir("install");
    let _ = fs::remove_dir_all(&tmp_dir);

    let output = run_install(&tmp_dir);

    assert_command_success(&output, "installing applet launchers");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Installed:"));

    for applet in &["cat", "chgrp", "chmod", "chown", "cp", "cut", "df", "du", "echo", "expr", "find", "free", "grep", "head", "id", "kill", "ln", "ls", "mkdir", "mv", "ps", "readlink", "relax", "rm", "sort", "su", "tail", "test", "[", "touch", "tr", "uname", "uniq", "uptime", "wc", "whoami"] {
        let launcher = installed_applet_path(&tmp_dir, applet);
        assert!(launcher.exists(), "launcher for {} should exist", applet);
        let meta = fs::symlink_metadata(&launcher).unwrap();

        #[cfg(unix)]
        assert!(meta.file_type().is_symlink(), "{} should be a symlink", applet);

        #[cfg(windows)]
        assert!(
            meta.is_file(),
            "{} should be an executable launcher",
            applet
        );
    }

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_install_overwrites_existing() {
    let tmp_dir = install_test_dir("install_overwrite");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let echo_launcher = installed_applet_path(&tmp_dir, "echo");
    fs::write(&echo_launcher, "dummy").unwrap();

    let output = run_install(&tmp_dir);
    assert_command_success(&output, "overwriting an existing launcher");

    let output = Command::new(&echo_launcher)
        .arg("overwritten")
        .output()
        .expect("failed to execute overwritten launcher");

    assert_command_success(&output, "running an overwritten launcher");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "overwritten");

    #[cfg(unix)]
    assert!(
        fs::symlink_metadata(&echo_launcher)
            .unwrap()
            .file_type()
            .is_symlink()
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_install_does_not_replace_directory() {
    let tmp_dir = install_test_dir("install_directory_conflict");
    let _ = fs::remove_dir_all(&tmp_dir);

    let echo_launcher = installed_applet_path(&tmp_dir, "echo");
    fs::create_dir_all(&echo_launcher).unwrap();

    let output = run_install(&tmp_dir);

    assert!(!output.status.success());
    assert!(echo_launcher.is_dir(), "an existing directory must be preserved");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("because it is a directory"),
        "the failure should explain the directory conflict"
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_install_creates_directory() {
    let tmp_dir = install_test_dir("install_newdir").join("sub");
    let _ = fs::remove_dir_all(tmp_dir.parent().unwrap());

    let output = run_install(&tmp_dir);

    assert_command_success(&output, "installing into a new directory");
    assert!(tmp_dir.exists());
    assert!(installed_applet_path(&tmp_dir, "echo").exists());

    let _ = fs::remove_dir_all(tmp_dir.parent().unwrap());
}

#[test]
fn test_install_launcher_invokes_applet() {
    let tmp_dir = install_test_dir("install_invoke");
    let _ = fs::remove_dir_all(&tmp_dir);

    let output = run_install(&tmp_dir);
    assert_command_success(&output, "installing an invokable launcher");

    let output = Command::new(installed_applet_path(&tmp_dir, "echo"))
        .args(["hello", "from", "launcher"])
        .output()
        .expect("failed to execute via installed launcher");

    assert_command_success(&output, "running an installed launcher");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello from launcher");

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

#[test]
#[cfg(unix)]
fn test_chmod_octal_mode() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_chmod");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("testfile.txt");
    fs::write(&file, "hello").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "chmod", "755", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let metadata = fs::metadata(&file).unwrap();
    let mode = metadata.permissions().mode() & 0o777;
    assert_eq!(mode, 0o755);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_chmod_multiple_files() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_chmod_multi");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let f1 = tmp_dir.join("a.txt");
    let f2 = tmp_dir.join("b.txt");
    fs::write(&f1, "one").unwrap();
    fs::write(&f2, "two").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "chmod", "0644", f1.to_str().unwrap(), f2.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(fs::metadata(&f1).unwrap().permissions().mode() & 0o777, 0o644);
    assert_eq!(fs::metadata(&f2).unwrap().permissions().mode() & 0o777, 0o644);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_chmod_recursive() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_chmod_r");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let sub = tmp_dir.join("subdir");
    fs::create_dir_all(sub.join("nested")).unwrap();
    let f1 = tmp_dir.join("file.txt");
    let f2 = sub.join("file2.txt");
    let f3 = sub.join("nested").join("file3.txt");
    fs::write(&f1, "a").unwrap();
    fs::write(&f2, "b").unwrap();
    fs::write(&f3, "c").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "chmod", "-R", "700", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(fs::metadata(&f1).unwrap().permissions().mode() & 0o777, 0o700);
    assert_eq!(fs::metadata(&f2).unwrap().permissions().mode() & 0o777, 0o700);
    assert_eq!(fs::metadata(&f3).unwrap().permissions().mode() & 0o777, 0o700);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(target_os = "linux")]
fn test_df_human_readable() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "df", "-h", "/"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Filesystem"));
    assert!(stdout.contains("Size"));
    assert!(stdout.contains("Used"));
    assert!(stdout.contains("Avail"));
    assert!(stdout.contains("Use%"));
    assert!(stdout.contains("Mounted on"));
    assert!(stdout.contains("/"));
}

#[test]
#[cfg(target_os = "linux")]
fn test_df_specific_path() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "df", "-h", "/tmp"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Filesystem"));
    assert!(stdout.contains("Mounted on"));
}

#[test]
#[cfg(target_os = "linux")]
fn test_df_no_args() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "df"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Filesystem"));
}

#[test]
fn test_du_summarize() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_du_s");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    fs::write(tmp_dir.join("file1.txt"), "hello world").unwrap();
    fs::create_dir(tmp_dir.join("sub")).unwrap();
    fs::write(tmp_dir.join("sub").join("file2.txt"), "more content here").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "du", "-s", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains(tmp_dir.to_str().unwrap()));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_du_human_readable() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_du_h");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    fs::write(tmp_dir.join("file1.txt"), "hello world").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "du", "-h", "-s", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("K") || stdout.contains("M") || stdout.contains("B"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_du_max_depth() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_du_d");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    fs::create_dir_all(tmp_dir.join("a").join("b")).unwrap();
    fs::write(tmp_dir.join("file.txt"), "data").unwrap();
    fs::write(tmp_dir.join("a").join("file2.txt"), "data2").unwrap();
    fs::write(tmp_dir.join("a").join("b").join("file3.txt"), "data3").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "du", "-h", "-d", "1", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert!(lines.len() >= 2, "expected at least 2 lines (subdir + total), got: {:?}", lines);
    let last_line = lines[lines.len() - 1];
    assert!(last_line.contains(tmp_dir.to_str().unwrap()), "last line should be the total for root dir");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(target_os = "linux")]
fn test_ps_basic() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "ps", "-e"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PID"));
    assert!(stdout.contains("TTY"));
    assert!(stdout.contains("STAT"));
    assert!(stdout.contains("TIME"));
    assert!(stdout.contains("COMMAND"));
}

#[test]
#[cfg(target_os = "linux")]
fn test_ps_shows_own_pid() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "ps", "-e"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let my_pid = std::process::id();
    assert!(stdout.contains(&my_pid.to_string()));
}

#[test]
#[cfg(target_os = "linux")]
fn test_ps_custom_columns() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "ps", "-e", "-o", "pid,cmd"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PID"));
    assert!(stdout.contains("COMMAND"));
    assert!(!stdout.contains("TTY"));
}

#[test]
#[cfg(unix)]
fn test_kill_list_signals() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "kill", "-l"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SIG"));
    assert!(stdout.contains("HUP"));
    assert!(stdout.contains("KILL"));
    assert!(stdout.contains("TERM"));
    assert!(stdout.contains("INT"));
}

#[test]
#[cfg(unix)]
fn test_kill_send_signal_to_child() {
    let mut child = Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("failed to spawn child");

    let pid = child.id() as i32;

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "kill", "-TERM", &pid.to_string()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());

    let status = child.wait().expect("failed to wait on child");
    assert!(!status.success());
}

#[test]
#[cfg(unix)]
fn test_kill_by_number() {
    let mut child = Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("failed to spawn child");

    let pid = child.id() as i32;

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "kill", "-9", &pid.to_string()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());

    let status = child.wait().expect("failed to wait on child");
    assert!(!status.success());
}

#[test]
#[cfg(target_os = "linux")]
fn test_free_basic() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "free"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("total"));
    assert!(stdout.contains("used"));
    assert!(stdout.contains("free"));
    assert!(stdout.contains("Mem:"));
    assert!(stdout.contains("Swap:"));
}

#[test]
#[cfg(target_os = "linux")]
fn test_free_human_readable() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "free", "-h"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Mem:"));
    assert!(stdout.contains("Swap:"));
    assert!(stdout.contains("K") || stdout.contains("M") || stdout.contains("G"));
}

#[test]
#[cfg(target_os = "linux")]
fn test_uptime_basic() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "uptime"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("up"));
    assert!(stdout.contains("load average:"));
    assert!(stdout.contains("user"));
}

#[test]
#[cfg(target_os = "linux")]
fn test_uptime_load_average_format() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "uptime"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.split("load average:").collect();
    assert_eq!(parts.len(), 2);
    let loads: Vec<&str> = parts[1].trim().split(',').collect();
    assert_eq!(loads.len(), 3);
    for load in &loads {
        let trimmed = load.trim();
        assert!(trimmed.contains('.'), "load value should be decimal: {}", trimmed);
    }
}

#[test]
fn test_ln_symbolic_link() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_ln_s");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let src = tmp_dir.join("source.txt");
    fs::write(&src, "hello").unwrap();
    let link = tmp_dir.join("link.txt");

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "ln", "-s", src.to_str().unwrap(), link.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(fs::read_to_string(&link).unwrap(), "hello");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_ln_hard_link() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_ln_hard");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let src = tmp_dir.join("source.txt");
    fs::write(&src, "hello").unwrap();
    let link = tmp_dir.join("link.txt");

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "ln", src.to_str().unwrap(), link.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(link.exists());
    assert!(!link.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(fs::read_to_string(&link).unwrap(), "hello");

    let src_meta = fs::metadata(&src).unwrap();
    let link_meta = fs::metadata(&link).unwrap();
    assert_eq!(src_meta.ino(), link_meta.ino());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_ln_force_overwrite() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_ln_f");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let src = tmp_dir.join("source.txt");
    fs::write(&src, "new content").unwrap();
    let link = tmp_dir.join("link.txt");
    fs::write(&link, "old content").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "ln", "-sf", src.to_str().unwrap(), link.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(fs::read_to_string(&link).unwrap(), "new content");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_ln_multiple_to_dir() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_ln_multi");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let f1 = tmp_dir.join("f1.txt");
    let f2 = tmp_dir.join("f2.txt");
    let dir = tmp_dir.join("links");
    fs::write(&f1, "one").unwrap();
    fs::write(&f2, "two").unwrap();
    fs::create_dir(&dir).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "ln", "-s",
            f1.to_str().unwrap(),
            f2.to_str().unwrap(),
            dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(dir.join("f1.txt").symlink_metadata().unwrap().file_type().is_symlink());
    assert!(dir.join("f2.txt").symlink_metadata().unwrap().file_type().is_symlink());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_readlink_symbolic() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_readlink");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let src = tmp_dir.join("target.txt");
    fs::write(&src, "data").unwrap();
    let link = tmp_dir.join("link.txt");
    std::os::unix::fs::symlink(&src, &link).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "readlink", link.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, src.to_str().unwrap());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_readlink_canonicalize() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_readlink_f");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let src = tmp_dir.join("target.txt");
    fs::write(&src, "data").unwrap();
    let link = tmp_dir.join("link.txt");
    std::os::unix::fs::symlink(&src, &link).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "readlink", "-f", link.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(stdout.starts_with('/'));
    assert!(stdout.ends_with("target.txt"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_readlink_no_newline() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_readlink_n");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let src = tmp_dir.join("target.txt");
    fs::write(&src, "data").unwrap();
    let link = tmp_dir.join("link.txt");
    std::os::unix::fs::symlink(&src, &link).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "readlink", "-n", link.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.ends_with('\n'));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_uname_sysname() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "uname", "-s"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(!stdout.is_empty(), "uname -s should output a non-empty sysname");
}

#[test]
#[cfg(unix)]
fn test_uname_all() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "uname", "-a"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().split_whitespace().collect();
    assert!(parts.len() >= 3, "uname -a should output at least 3 fields, got: {:?}", parts);
}

#[test]
#[cfg(unix)]
fn test_uname_default_is_sysname() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "uname"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(!stdout.is_empty(), "uname should output a non-empty sysname");
}

#[test]
#[cfg(not(unix))]
fn test_uname_sysname() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "uname", "-s"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(!stdout.is_empty());
}

#[test]
#[cfg(not(unix))]
fn test_uname_all() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "uname", "-a"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().split_whitespace().collect();
    assert!(parts.len() >= 1, "uname -a should output at least 1 field");
}

#[test]
#[cfg(not(unix))]
fn test_uname_default_is_sysname() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "uname"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(!stdout.is_empty());
}

#[test]
fn test_test_file_exists() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_test");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("testfile.txt");
    fs::write(&file, "hello").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "test", "-f", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_test_directory() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_test_dir");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "test", "-d", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_bracket_numeric_equal() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "[", "1", "-eq", "1", "]"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
}

#[test]
fn test_bracket_string_equal() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "[", "a", "=", "b", "]"])
        .output()
        .expect("failed to execute process");

    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn test_test_string_zero_length() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "test", "-z", ""])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
}

#[test]
fn test_test_string_nonzero_length() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "test", "-n", "hello"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
}

#[test]
fn test_test_numeric_comparison() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "test", "5", "-gt", "3"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
}

#[test]
fn test_test_logical_and() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "test", "1", "-eq", "1", "-a", "2", "-eq", "2"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
}

#[test]
fn test_test_logical_or() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "test", "1", "-eq", "2", "-o", "2", "-eq", "2"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
}

#[test]
fn test_test_logical_not() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "test", "!", "1", "-eq", "2"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
}

#[test]
fn test_test_file_size() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_test_size");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("testfile.txt");
    fs::write(&file, "hello").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "test", "-s", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_test_symlink() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_test_symlink");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("target.txt");
    fs::write(&file, "data").unwrap();
    let link = tmp_dir.join("link.txt");
    std::os::unix::fs::symlink(&file, &link).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "test", "-L", link.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_expr_addition() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "expr", "5", "+", "3"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "8");
}

#[test]
fn test_expr_multiplication() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "expr", "10", "*", "2"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "20");
}

#[test]
fn test_expr_length() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "expr", "length", "hello"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "5");
}

#[test]
fn test_expr_substring() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "expr", "substr", "hello", "2", "3"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ell");
}

#[test]
fn test_expr_comparison() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "expr", "5", ">", "3"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
}

#[test]
fn test_expr_logical_or() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "expr", "0", "|", "5"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "5");
}

#[test]
fn test_expr_logical_and() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "expr", "3", "&", "5"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "3");
}

#[test]
fn test_expr_division_by_zero() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "expr", "10", "/", "0"])
        .output()
        .expect("failed to execute process");

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_expr_modulo() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "expr", "10", "%", "3"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
}

#[test]
fn test_expr_string_equality() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "expr", "hello", "=", "hello"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
}

#[test]
fn test_find_name_pattern() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_find");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    fs::write(tmp_dir.join("file1.rs"), "code").unwrap();
    fs::write(tmp_dir.join("file2.txt"), "text").unwrap();
    fs::write(tmp_dir.join("file3.rs"), "more code").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "find", tmp_dir.to_str().unwrap(), "-name", "*.rs"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("file1.rs"));
    assert!(stdout.contains("file3.rs"));
    assert!(!stdout.contains("file2.txt"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_find_type_directory() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_find_type");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    fs::create_dir(tmp_dir.join("subdir1")).unwrap();
    fs::create_dir(tmp_dir.join("subdir2")).unwrap();
    fs::write(tmp_dir.join("file.txt"), "content").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "find", tmp_dir.to_str().unwrap(), "-type", "d", "-maxdepth", "1"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("subdir1"));
    assert!(stdout.contains("subdir2"));
    assert!(!stdout.contains("file.txt"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_find_maxdepth() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_find_depth");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    fs::create_dir_all(tmp_dir.join("a").join("b").join("c")).unwrap();
    fs::write(tmp_dir.join("file1.txt"), "content").unwrap();
    fs::write(tmp_dir.join("a").join("file2.txt"), "content").unwrap();
    fs::write(tmp_dir.join("a").join("b").join("file3.txt"), "content").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "find", tmp_dir.to_str().unwrap(), "-name", "*.txt", "-maxdepth", "1"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("file1.txt"));
    assert!(!stdout.contains("file2.txt"));
    assert!(!stdout.contains("file3.txt"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_find_empty() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_find_empty");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    fs::write(tmp_dir.join("empty.txt"), "").unwrap();
    fs::write(tmp_dir.join("nonempty.txt"), "content").unwrap();
    fs::create_dir(tmp_dir.join("emptydir")).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "find", tmp_dir.to_str().unwrap(), "-empty"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("empty.txt"));
    assert!(stdout.contains("emptydir"));
    assert!(!stdout.contains("nonempty.txt"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_find_default_current_directory() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "find"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("."));
}

#[test]
fn test_find_type_file() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_find_type_f");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    fs::write(tmp_dir.join("file1.txt"), "content").unwrap();
    fs::write(tmp_dir.join("file2.txt"), "content").unwrap();
    fs::create_dir(tmp_dir.join("subdir")).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "find", tmp_dir.to_str().unwrap(), "-type", "f"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
    assert!(!stdout.contains("subdir"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_find_symlink() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_find_symlink");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("target.txt");
    fs::write(&file, "data").unwrap();
    let link = tmp_dir.join("link.txt");
    std::os::unix::fs::symlink(&file, &link).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "find", tmp_dir.to_str().unwrap(), "-type", "l"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("link.txt"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_wc_lines() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_wc_l");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "line1\nline2\nline3\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "wc", "-l", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("3"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_wc_words() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_wc_w");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "hello world\nfoo bar baz\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "wc", "-w", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("5"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_wc_stdin() {
    let mut child = Command::new("cargo")
        .args(["run", "--quiet", "--", "wc", "-l"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn process");

    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin.write_all(b"line1\nline2\nline3\nline4\n").unwrap();
    }

    let output = child.wait_with_output().expect("failed to wait");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("4"));
}

#[test]
fn test_wc_multiple_files() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_wc_multi");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let f1 = tmp_dir.join("a.txt");
    let f2 = tmp_dir.join("b.txt");
    fs::write(&f1, "one\ntwo\n").unwrap();
    fs::write(&f2, "three\nfour\nfive\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "wc", "-l", f1.to_str().unwrap(), f2.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("total"));
    assert!(stdout.contains("5"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_sort_basic() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_sort");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "cherry\napple\nbanana\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "sort", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, vec!["apple", "banana", "cherry"]);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_sort_numeric_reverse() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_sort_nr");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "10\n2\n30\n1\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "sort", "-n", "-r", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, vec!["30", "10", "2", "1"]);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_sort_unique() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_sort_u");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "banana\napple\nbanana\ncherry\napple\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "sort", "-u", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, vec!["apple", "banana", "cherry"]);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_sort_stdin() {
    let mut child = Command::new("cargo")
        .args(["run", "--quiet", "--", "sort"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn process");

    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin.write_all(b"cherry\napple\nbanana\n").unwrap();
    }

    let output = child.wait_with_output().expect("failed to wait");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, vec!["apple", "banana", "cherry"]);
}

#[test]
fn test_uniq_count() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_uniq_c");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "a\na\nb\nb\nb\nc\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "uniq", "-c", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("2"));
    assert!(lines[0].contains("a"));
    assert!(lines[1].contains("3"));
    assert!(lines[1].contains("b"));
    assert!(lines[2].contains("1"));
    assert!(lines[2].contains("c"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_uniq_repeated() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_uniq_d");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "a\na\nb\nc\nc\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "uniq", "-d", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, vec!["a", "c"]);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_uniq_unique() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_uniq_u");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "a\na\nb\nc\nc\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "uniq", "-u", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, vec!["b"]);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_uniq_ignore_case() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_uniq_i");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "Hello\nhello\nHELLO\nWorld\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "uniq", "-i", "-c", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("3"));
    assert!(lines[1].contains("1"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_cut_fields_csv() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cut_f");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.csv");
    fs::write(&file, "name,age,city\nAlice,30,NYC\nBob,25,LA\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "cut", "-d", ",", "-f", "1,2", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "name,age");
    assert_eq!(lines[1], "Alice,30");
    assert_eq!(lines[2], "Bob,25");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_cut_characters() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cut_c");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "Hello World\nFoo Bar\n").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "cut", "-c", "1-5", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "Hello");
    assert_eq!(lines[1], "Foo B");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_cut_stdin() {
    let mut child = Command::new("cargo")
        .args(["run", "--quiet", "--", "cut", "-d", ":", "-f", "1"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn process");

    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin.write_all(b"user:x:1000\nroot:x:0\n").unwrap();
    }

    let output = child.wait_with_output().expect("failed to wait");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "user");
    assert_eq!(lines[1], "root");
}

#[test]
fn test_tr_translate() {
    let mut child = Command::new("cargo")
        .args(["run", "--quiet", "--", "tr", "a-z", "A-Z"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn process");

    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin.write_all(b"hello world\n").unwrap();
    }

    let output = child.wait_with_output().expect("failed to wait");
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "HELLO WORLD\n");
}

#[test]
fn test_tr_delete() {
    let mut child = Command::new("cargo")
        .args(["run", "--quiet", "--", "tr", "-d", "0-9"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn process");

    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin.write_all(b"abc123def456\n").unwrap();
    }

    let output = child.wait_with_output().expect("failed to wait");
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "abcdef\n");
}

#[test]
fn test_tr_squeeze() {
    let mut child = Command::new("cargo")
        .args(["run", "--quiet", "--", "tr", "-s", " "])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn process");

    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin.write_all(b"hello    world\n").unwrap();
    }

    let output = child.wait_with_output().expect("failed to wait");
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello world\n");
}

#[test]
fn test_whoami_nonempty() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "whoami"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(!stdout.is_empty(), "whoami should output a non-empty username");
}

#[test]
#[cfg(unix)]
fn test_whoami_matches_id_un() {
    let whoami_output = Command::new("cargo")
        .args(["run", "--quiet", "--", "whoami"])
        .output()
        .expect("failed to execute process");

    let id_output = Command::new("cargo")
        .args(["run", "--quiet", "--", "id", "-u", "-n"])
        .output()
        .expect("failed to execute process");

    assert!(whoami_output.status.success());
    assert!(id_output.status.success());
    let whoami_name = String::from_utf8_lossy(&whoami_output.stdout).trim().to_string();
    let id_name = String::from_utf8_lossy(&id_output.stdout).trim().to_string();
    assert_eq!(whoami_name, id_name);
}

#[test]
#[cfg(unix)]
fn test_id_default_format() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "id"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("uid="));
    assert!(stdout.contains("gid="));
    assert!(stdout.contains("groups="));
}

#[test]
#[cfg(unix)]
fn test_id_u_flag() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "id", "-u"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(stdout.parse::<u32>().is_ok(), "id -u should output a numeric UID, got: {}", stdout);
}

#[test]
#[cfg(unix)]
fn test_id_u_name_flag() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "id", "-u", "-n"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(!stdout.is_empty(), "id -u -n should output a non-empty username");
    assert!(stdout.parse::<u32>().is_err(), "id -u -n should output a name, not a number");
}

#[test]
#[cfg(unix)]
fn test_id_g_flag() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "id", "-g"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(stdout.parse::<u32>().is_ok(), "id -g should output a numeric GID, got: {}", stdout);
}

#[test]
#[cfg(unix)]
fn test_id_g_name_flag() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "id", "-g", "-n"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(!stdout.is_empty(), "id -g -n should output a non-empty group name");
}

#[test]
#[cfg(unix)]
fn test_id_g_supplementary_flag() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "id", "-G"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        let gids: Vec<&str> = stdout.split_whitespace().collect();
        for gid in &gids {
            assert!(gid.parse::<u32>().is_ok(), "each group should be numeric, got: {}", gid);
        }
    }
}

#[test]
#[cfg(unix)]
fn test_id_nonexistent_user() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "id", "nonexistent_user_xyz_12345"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no such user"));
}

#[test]
#[cfg(unix)]
fn test_id_combined_flags() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "id", "-un"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(!stdout.is_empty(), "id -un should output a username");
    assert!(stdout.parse::<u32>().is_err(), "id -un should output a name, not a number");
}

#[test]
#[cfg(unix)]
fn test_chown_missing_operand() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "chown"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing operand"));
}

#[test]
#[cfg(unix)]
fn test_chown_invalid_user() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_chown_inv");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("testfile.txt");
    fs::write(&file, "hello").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "chown", "nonexistent_user_xyz_12345", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid user"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_chown_no_file() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "chown", "root", "/tmp/idlebox_nonexistent_file_xyz"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot access"));
}

#[test]
#[cfg(unix)]
fn test_chgrp_missing_operand() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "chgrp"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing operand"));
}

#[test]
#[cfg(unix)]
fn test_chgrp_invalid_group() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_chgrp_inv");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("testfile.txt");
    fs::write(&file, "hello").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "chgrp", "nonexistent_group_xyz_12345", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid group"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_chgrp_no_file() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "chgrp", "0", "/tmp/idlebox_nonexistent_file_xyz"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot access") || stderr.contains("No such file"));
}

#[test]
#[cfg(unix)]
fn test_su_missing_command_argument() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "su", "-c"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("requires an argument"));
}

#[test]
#[cfg(unix)]
fn test_su_nonexistent_user() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "su", "nonexistent_user_xyz_12345"])
        .output()
        .expect("failed to execute process");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not exist") || stderr.contains("permission denied"),
        "su should report either non-existent user or permission denied, got: {}",
        stderr
    );
}

#[test]
#[cfg(unix)]
fn test_su_help() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "su", "--help"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("su"));
}
