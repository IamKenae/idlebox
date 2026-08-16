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
    run_install_with_options(directory, &[])
}

fn run_install_with_options(directory: &Path, options: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_idlebox"))
        .arg("--install")
        .args(options)
        .arg(directory)
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

fn archive_test_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "idlebox_test_archive_{}_{}",
        name,
        std::process::id()
    ))
}

fn append_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn write_zip_fixture(path: &Path, entries: &[(&str, &[u8], u16)]) {
    struct Record {
        name: Vec<u8>,
        crc: u32,
        compressed_size: u32,
        uncompressed_size: u32,
        method: u16,
        local_offset: u32,
    }

    let mut archive = Vec::new();
    let mut records = Vec::new();

    for &(name, data, method) in entries {
        let compressed = match method {
            0 => data.to_vec(),
            8 => {
                let mut encoder =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(data).unwrap();
                encoder.finish().unwrap()
            }
            _ => panic!("unsupported fixture compression method"),
        };
        let name = name.as_bytes().to_vec();
        let record = Record {
            name: name.clone(),
            crc: crc32fast::hash(data),
            compressed_size: compressed.len().try_into().unwrap(),
            uncompressed_size: data.len().try_into().unwrap(),
            method,
            local_offset: archive.len().try_into().unwrap(),
        };

        append_u32(&mut archive, 0x0403_4b50);
        append_u16(&mut archive, 20);
        append_u16(&mut archive, 0x0800);
        append_u16(&mut archive, method);
        append_u16(&mut archive, 0);
        append_u16(&mut archive, 0);
        append_u32(&mut archive, record.crc);
        append_u32(&mut archive, record.compressed_size);
        append_u32(&mut archive, record.uncompressed_size);
        append_u16(&mut archive, name.len().try_into().unwrap());
        append_u16(&mut archive, 0);
        archive.extend_from_slice(&name);
        archive.extend_from_slice(&compressed);
        records.push(record);
    }

    let central_offset: u32 = archive.len().try_into().unwrap();
    for record in &records {
        append_u32(&mut archive, 0x0201_4b50);
        append_u16(&mut archive, 20);
        append_u16(&mut archive, 20);
        append_u16(&mut archive, 0x0800);
        append_u16(&mut archive, record.method);
        append_u16(&mut archive, 0);
        append_u16(&mut archive, 0);
        append_u32(&mut archive, record.crc);
        append_u32(&mut archive, record.compressed_size);
        append_u32(&mut archive, record.uncompressed_size);
        append_u16(&mut archive, record.name.len().try_into().unwrap());
        append_u16(&mut archive, 0);
        append_u16(&mut archive, 0);
        append_u16(&mut archive, 0);
        append_u16(&mut archive, 0);
        append_u32(&mut archive, 0);
        append_u32(&mut archive, record.local_offset);
        archive.extend_from_slice(&record.name);
    }
    let central_size = u32::try_from(archive.len()).unwrap() - central_offset;

    append_u32(&mut archive, 0x0605_4b50);
    append_u16(&mut archive, 0);
    append_u16(&mut archive, 0);
    append_u16(&mut archive, records.len().try_into().unwrap());
    append_u16(&mut archive, records.len().try_into().unwrap());
    append_u32(&mut archive, central_size);
    append_u32(&mut archive, central_offset);
    append_u16(&mut archive, 0);

    fs::write(path, archive).unwrap();
}

#[test]
fn test_echo_basic() {
    let output = idlebox_command()
        .args(["echo", "hello", "world"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello world"
    );
}

#[test]
fn test_echo_no_newline() {
    let output = idlebox_command()
        .args(["echo", "-n", "test"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "test");
}

#[test]
fn test_unknown_applet() {
    let output = idlebox_command()
        .args(["nonexistent"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("applet not found"));
    assert!(stderr.contains("idlebox list"));
}

#[test]
fn test_global_help() {
    let output = idlebox_command()
        .arg("--help")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("A lightweight multi-call toolbox"));
    assert!(stdout.contains("idlebox help [APPLET]"));
    assert!(stdout.contains("idlebox --version"));
}

#[test]
fn test_global_version() {
    let output = idlebox_command()
        .arg("--version")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("idlebox {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn test_help_subcommand_for_applet() {
    let output = idlebox_command()
        .args(["help", "cat"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: cat"));
    assert!(stdout.contains("Concatenate files"));
}

#[test]
fn test_help_after_option_separator_is_applet_input() {
    let output = idlebox_command()
        .args(["echo", "--", "--help"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "-- --help\n");
}

#[test]
fn test_list_applets() {
    let output = idlebox_command()
        .args(["list"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("basename"));
    assert!(stdout.contains("cat"));
    assert!(stdout.contains("chgrp"));
    assert!(stdout.contains("chmod"));
    assert!(stdout.contains("chown"));
    assert!(stdout.contains("cp"));
    assert!(stdout.contains("cut"));
    assert!(stdout.contains("df"));
    assert!(stdout.contains("dirname"));
    assert!(stdout.contains("du"));
    assert!(stdout.contains("echo"));
    assert!(stdout.contains("env"));
    assert!(stdout.contains("expr"));
    assert!(stdout.contains("false"));
    assert!(stdout.contains("find"));
    assert!(stdout.contains("free"));
    assert!(stdout.contains("grep"));
    assert!(stdout.contains("gunzip"));
    assert!(stdout.contains("gzip"));
    assert!(stdout.contains("head"));
    assert!(stdout.contains("id"));
    assert!(stdout.contains("kill"));
    assert!(stdout.contains("ln"));
    assert!(stdout.contains("ls"));
    assert!(stdout.contains("mkdir"));
    assert!(stdout.contains("mv"));
    assert!(stdout.contains("ps"));
    assert!(stdout.contains("printf"));
    assert!(stdout.contains("printenv"));
    assert!(stdout.contains("pwd"));
    assert!(stdout.contains("readlink"));
    assert!(stdout.contains("realpath"));
    assert!(stdout.contains("relax"));
    assert!(stdout.contains("rm"));
    assert!(stdout.contains("sort"));
    assert!(stdout.contains("sleep"));
    assert!(stdout.contains("su"));
    assert!(stdout.contains("tail"));
    assert!(stdout.contains("tar"));
    assert!(stdout.contains("tee"));
    assert!(stdout.contains("test"));
    assert!(stdout.contains("touch"));
    assert!(stdout.contains("tr"));
    assert!(stdout.contains("tree"));
    assert!(stdout.contains("true"));
    assert!(stdout.contains("uname"));
    assert!(stdout.contains("uniq"));
    assert!(stdout.contains("unzip"));
    assert!(stdout.contains("uptime"));
    assert!(stdout.contains("wc"));
    assert!(stdout.contains("whoami"));
    assert!(stdout.contains("zcat"));
}

#[test]
fn test_list_long_alias_matches_list() {
    let list = idlebox_command()
        .arg("list")
        .output()
        .expect("failed to execute process");
    let long_alias = idlebox_command()
        .arg("--list")
        .output()
        .expect("failed to execute process");

    assert!(list.status.success());
    assert!(long_alias.status.success());
    assert_eq!(list.stdout, long_alias.stdout);
}

#[test]
fn test_list_rejects_extra_arguments() {
    let output = idlebox_command()
        .args(["list", "extra"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected argument 'extra'"));
}

#[test]
fn test_install_help() {
    let output = idlebox_command()
        .args(["--install", "--help"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: idlebox --install [OPTIONS] [PATH]"));
    assert!(stdout.contains("--force"));
    assert!(stdout.contains("--dry-run"));
}

#[test]
fn test_install_rejects_extra_arguments() {
    let output = idlebox_command()
        .args(["--install", "first", "second"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected argument 'second'"));
}

#[test]
fn test_install_requires_separator_for_dash_prefixed_path() {
    let output = idlebox_command()
        .args(["--install", "-tools"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown option '-tools'"));
    assert!(stderr.contains("Use '--' before PATH"));
}

#[test]
fn test_help_short_flag() {
    let output = idlebox_command()
        .args(["relax", "-h"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("relax"));
}

#[test]
fn test_help_long_flag() {
    let output = idlebox_command()
        .args(["echo", "--help"])
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

    let output = idlebox_command()
        .args(["cat", test_file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["cat", "-n", test_file.to_str().unwrap()])
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
    let mut child = idlebox_command()
        .args(["cat"])
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

    let output = idlebox_command()
        .args(["ls", tmp_dir.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["ls", "-l", tmp_dir.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["ls", "-a", tmp_dir.to_str().unwrap()])
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
    assert!(stdout.contains("58 installed, 0 updated, 0 already installed"));
    assert!(stdout.contains("Tip: add"));

    for applet in &[
        "basename", "cat", "chgrp", "chmod", "chown", "cp", "cut", "df", "dirname", "du", "echo",
        "env", "expr", "false", "find", "free", "grep", "gunzip", "gzip", "head", "id", "kill",
        "ln", "ls", "mkdir", "mv", "ps", "printf", "printenv", "pwd", "readlink", "realpath",
        "relax", "rm", "sleep", "sort", "su", "tail", "tar", "tee", "test", "[", "touch", "tr",
        "tree", "true", "uname", "uniq", "unzip", "uptime", "wc", "whoami", "zcat",
    ] {
        let launcher = installed_applet_path(&tmp_dir, applet);
        assert!(launcher.exists(), "launcher for {} should exist", applet);
        let meta = fs::symlink_metadata(&launcher).unwrap();

        #[cfg(unix)]
        assert!(
            meta.file_type().is_symlink(),
            "{} should be a symlink",
            applet
        );

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
fn test_install_refuses_existing_file_without_force() {
    let tmp_dir = install_test_dir("install_refuse_existing");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let echo_launcher = installed_applet_path(&tmp_dir, "echo");
    let cat_launcher = installed_applet_path(&tmp_dir, "cat");
    fs::write(&echo_launcher, "dummy").unwrap();
    fs::write(&cat_launcher, "also dummy").unwrap();

    let output = run_install(&tmp_dir);

    assert!(!output.status.success());
    assert_eq!(fs::read_to_string(&echo_launcher).unwrap(), "dummy");
    assert_eq!(fs::read_to_string(&cat_launcher).unwrap(), "also dummy");
    assert!(
        !installed_applet_path(&tmp_dir, "chgrp").exists(),
        "preflight must finish before any launcher is written"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Conflict:"));
    assert!(stderr.contains(&echo_launcher.display().to_string()));
    assert!(stderr.contains(&cat_launcher.display().to_string()));
    assert!(stderr.contains("--force"));
    assert!(stderr.contains("no launchers were changed"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_install_force_overwrites_existing_file() {
    let tmp_dir = install_test_dir("install_force_overwrite");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let echo_launcher = installed_applet_path(&tmp_dir, "echo");
    fs::write(&echo_launcher, "dummy").unwrap();

    let output = run_install_with_options(&tmp_dir, &["--force"]);
    assert_command_success(&output, "force-updating an existing launcher");
    assert!(String::from_utf8_lossy(&output.stdout).contains("Updated:"));

    let output = Command::new(&echo_launcher)
        .arg("overwritten")
        .output()
        .expect("failed to execute overwritten launcher");

    assert_command_success(&output, "running an overwritten launcher");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "overwritten"
    );

    #[cfg(unix)]
    assert!(fs::symlink_metadata(&echo_launcher)
        .unwrap()
        .file_type()
        .is_symlink());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_install_does_not_replace_directory() {
    let tmp_dir = install_test_dir("install_directory_conflict");
    let _ = fs::remove_dir_all(&tmp_dir);

    let echo_launcher = installed_applet_path(&tmp_dir, "echo");
    fs::create_dir_all(&echo_launcher).unwrap();

    let output = run_install_with_options(&tmp_dir, &["--force"]);

    assert!(!output.status.success());
    assert!(
        echo_launcher.is_dir(),
        "an existing directory must be preserved"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("is a directory; directories are never replaced"),
        "the failure should explain the directory conflict"
    );
    assert!(
        !installed_applet_path(&tmp_dir, "cat").exists(),
        "a directory conflict must abort before installing other launchers"
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_install_dry_run_does_not_create_directory() {
    let tmp_dir = install_test_dir("install_dry_run");
    let _ = fs::remove_dir_all(&tmp_dir);

    let output = run_install_with_options(&tmp_dir, &["--dry-run"]);

    assert_command_success(&output, "previewing an installation");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Would install:"));
    assert!(stdout.contains("no changes made"));
    assert!(
        !tmp_dir.exists(),
        "a dry run must not create the target directory"
    );
}

#[test]
fn test_install_force_dry_run_does_not_replace_file() {
    let tmp_dir = install_test_dir("install_force_dry_run");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let echo_launcher = installed_applet_path(&tmp_dir, "echo");
    fs::write(&echo_launcher, "dummy").unwrap();

    let output = run_install_with_options(&tmp_dir, &["--force", "--dry-run"]);

    assert_command_success(&output, "previewing a forced update");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Would update:"));
    assert!(stdout.contains("1 to update"));
    assert_eq!(fs::read_to_string(&echo_launcher).unwrap(), "dummy");
    assert!(!installed_applet_path(&tmp_dir, "cat").exists());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_install_rerun_skips_current_launchers() {
    let tmp_dir = install_test_dir("install_rerun");
    let _ = fs::remove_dir_all(&tmp_dir);

    let first = run_install(&tmp_dir);
    assert_command_success(&first, "installing launchers before a rerun");

    let second = run_install(&tmp_dir);
    assert_command_success(&second, "rerunning an installation");
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(stdout.contains("0 installed, 0 updated, 58 already installed"));

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
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello from launcher"
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_mkdir_basic() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_mkdir");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let target = tmp_dir.join("newdir");
    let output = idlebox_command()
        .args(["mkdir", target.to_str().unwrap()])
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
    let output = idlebox_command()
        .args(["mkdir", "-p", nested.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["mkdir", "-p", tmp_dir.to_str().unwrap()])
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
    let output = idlebox_command()
        .args(["mkdir", d1.to_str().unwrap(), d2.to_str().unwrap()])
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
    let output = idlebox_command()
        .args(["mkdir", nested.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["rm", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["rm", "-rf", sub.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(!sub.exists());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_rm_force_nonexistent() {
    let output = idlebox_command()
        .args(["rm", "-f", "/tmp/idlebox_nonexistent_file_xyz"])
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

    let output = idlebox_command()
        .args(["rm", sub.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["cp", src.to_str().unwrap(), dst.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(dst.exists());
    assert_eq!(fs::read_to_string(&dst).unwrap(), "copy me");
    assert!(src.exists());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_cp_force_overwrites_existing_file() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cp_force");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let src = tmp_dir.join("source.txt");
    let dst = tmp_dir.join("dest.txt");
    fs::write(&src, "new content").unwrap();
    fs::write(&dst, "old content").unwrap();

    let output = idlebox_command()
        .args(["cp", "-f", src.to_str().unwrap(), dst.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(fs::read_to_string(&src).unwrap(), "new content");
    assert_eq!(fs::read_to_string(&dst).unwrap(), "new content");
    assert!(fs::read_dir(&tmp_dir).unwrap().all(|entry| !entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .contains(".idlebox-")));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_cp_force_rejects_same_path_without_removing_source() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cp_force_same");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("same.txt");
    fs::write(&file, "keep me").unwrap();

    let output = idlebox_command()
        .args(["cp", "-f", file.to_str().unwrap(), file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("same file"));
    assert_eq!(fs::read_to_string(&file).unwrap(), "keep me");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_cp_force_rejects_hard_link_alias_without_breaking_links() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cp_force_hardlink");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let src = tmp_dir.join("source.txt");
    let alias = tmp_dir.join("alias.txt");
    fs::write(&src, "keep both").unwrap();
    fs::hard_link(&src, &alias).unwrap();

    let output = idlebox_command()
        .args(["cp", "-f", src.to_str().unwrap(), alias.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("same file"));
    assert_eq!(fs::read_to_string(&src).unwrap(), "keep both");
    assert_eq!(fs::read_to_string(&alias).unwrap(), "keep both");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_cp_rejects_copying_symlink_onto_itself_without_removing_it() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cp_symlink_same");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let link = tmp_dir.join("link");
    std::os::unix::fs::symlink("target", &link).unwrap();

    let output = idlebox_command()
        .args(["cp", link.to_str().unwrap(), link.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("same file"));
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(
        fs::read_link(&link).unwrap(),
        std::path::Path::new("target")
    );

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
    let output = idlebox_command()
        .args([
            "cp",
            "-r",
            src_dir.to_str().unwrap(),
            dst_dir.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(dst_dir.join("file1.txt").exists());
    assert!(dst_dir.join("sub").join("file2.txt").exists());
    assert_eq!(
        fs::read_to_string(dst_dir.join("file1.txt")).unwrap(),
        "one"
    );
    assert_eq!(
        fs::read_to_string(dst_dir.join("sub").join("file2.txt")).unwrap(),
        "two"
    );

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

    let output = idlebox_command()
        .args([
            "cp",
            f1.to_str().unwrap(),
            f2.to_str().unwrap(),
            dest.to_str().unwrap(),
        ])
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

    let output = idlebox_command()
        .args(["mv", src.to_str().unwrap(), dst.to_str().unwrap()])
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

    let output = idlebox_command()
        .args([
            "mv",
            f1.to_str().unwrap(),
            f2.to_str().unwrap(),
            dest.to_str().unwrap(),
        ])
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

    let output = idlebox_command()
        .args(["mv", src.to_str().unwrap(), dst.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(!src.exists());
    assert!(dst.join("nested").join("file.txt").exists());
    assert_eq!(
        fs::read_to_string(dst.join("nested").join("file.txt")).unwrap(),
        "data"
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_touch_create_file() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_touch");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("newfile.txt");
    let output = idlebox_command()
        .args(["touch", file.to_str().unwrap()])
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
    let output = idlebox_command()
        .args([
            "touch",
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
    let old_time = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1);
    fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_times(
            fs::FileTimes::new()
                .set_accessed(old_time)
                .set_modified(old_time),
        )
        .unwrap();

    let output = idlebox_command()
        .args(["touch", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(fs::read_to_string(&file).unwrap(), "content");
    assert!(fs::metadata(&file).unwrap().modified().unwrap() > old_time);

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

    let output = idlebox_command()
        .args(["head", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["head", "-n", "5", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["head", "-c", "5", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_head_stdin() {
    let mut child = idlebox_command()
        .args(["head", "-n", "3"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn process");

    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin
            .write_all(b"line1\nline2\nline3\nline4\nline5\n")
            .unwrap();
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

    let output = idlebox_command()
        .args(["tail", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["tail", "-n", "3", file.to_str().unwrap()])
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
    let mut child = idlebox_command()
        .args(["tail", "-n", "2"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn process");

    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin
            .write_all(b"line1\nline2\nline3\nline4\nline5\n")
            .unwrap();
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

    let output = idlebox_command()
        .args(["tail", "-c", "6", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["grep", "apple", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["grep", "-i", "error", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["grep", "-n", "gamma", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["grep", "-v", "apple", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["grep", "-c", "apple", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "2");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_grep_stdin() {
    let mut child = idlebox_command()
        .args(["grep", "-i", "error"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn process");

    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin
            .write_all(b"Info: ok\nError: fail\nWarning: maybe\nerror: again\n")
            .unwrap();
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

    let output = idlebox_command()
        .args(["grep", "-in", "error", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["grep", "zzz", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["chmod", "755", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["chmod", "0644", f1.to_str().unwrap(), f2.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(
        fs::metadata(&f1).unwrap().permissions().mode() & 0o777,
        0o644
    );
    assert_eq!(
        fs::metadata(&f2).unwrap().permissions().mode() & 0o777,
        0o644
    );

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

    let output = idlebox_command()
        .args(["chmod", "-R", "700", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(
        fs::metadata(&f1).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(&f2).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(&f3).unwrap().permissions().mode() & 0o777,
        0o700
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(target_os = "linux")]
fn test_df_human_readable() {
    let output = idlebox_command()
        .args(["df", "-h", "/"])
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
    let output = idlebox_command()
        .args(["df", "-h", "/tmp"])
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
    let output = idlebox_command()
        .args(["df"])
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

    let output = idlebox_command()
        .args(["du", "-s", tmp_dir.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["du", "-h", "-s", tmp_dir.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["du", "-h", "-d", "1", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert!(
        lines.len() >= 2,
        "expected at least 2 lines (subdir + total), got: {:?}",
        lines
    );
    let last_line = lines[lines.len() - 1];
    assert!(
        last_line.contains(tmp_dir.to_str().unwrap()),
        "last line should be the total for root dir"
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(target_os = "linux")]
fn test_ps_basic() {
    let output = idlebox_command()
        .args(["ps", "-e"])
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
    let output = idlebox_command()
        .args(["ps", "-e"])
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
    let output = idlebox_command()
        .args(["ps", "-e", "-o", "pid,cmd"])
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
    let output = idlebox_command()
        .args(["kill", "-l"])
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

    let output = idlebox_command()
        .args(["kill", "-TERM", &pid.to_string()])
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

    let output = idlebox_command()
        .args(["kill", "-9", &pid.to_string()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());

    let status = child.wait().expect("failed to wait on child");
    assert!(!status.success());
}

#[test]
#[cfg(target_os = "linux")]
fn test_free_basic() {
    let output = idlebox_command()
        .args(["free"])
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
    let output = idlebox_command()
        .args(["free", "-h"])
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
    let output = idlebox_command()
        .args(["uptime"])
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
    let output = idlebox_command()
        .args(["uptime"])
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
        assert!(
            trimmed.contains('.'),
            "load value should be decimal: {}",
            trimmed
        );
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

    let output = idlebox_command()
        .args(["ln", "-s", src.to_str().unwrap(), link.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["ln", src.to_str().unwrap(), link.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["ln", "-sf", src.to_str().unwrap(), link.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(fs::read_to_string(&link).unwrap(), "new content");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_ln_force_missing_source_preserves_existing_destination() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_ln_force_missing");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let missing = tmp_dir.join("missing.txt");
    let destination = tmp_dir.join("destination.txt");
    fs::write(&destination, "keep me").unwrap();

    let output = idlebox_command()
        .args([
            "ln",
            "-f",
            missing.to_str().unwrap(),
            destination.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    assert_eq!(fs::read_to_string(&destination).unwrap(), "keep me");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_ln_force_rejects_same_file_without_removing_it() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_ln_force_same");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("same.txt");
    fs::write(&file, "keep me").unwrap();

    let output = idlebox_command()
        .args(["ln", "-f", file.to_str().unwrap(), file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("same file"));
    assert_eq!(fs::read_to_string(&file).unwrap(), "keep me");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_ln_accepts_double_dash_for_dash_prefixed_paths() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_ln_double_dash");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    fs::write(tmp_dir.join("-source"), "content").unwrap();

    let output = idlebox_command()
        .current_dir(&tmp_dir)
        .args(["ln", "--", "-source", "-link"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(tmp_dir.join("-link")).unwrap(),
        "content"
    );

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

    let output = idlebox_command()
        .args([
            "ln",
            "-s",
            f1.to_str().unwrap(),
            f2.to_str().unwrap(),
            dir.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(dir
        .join("f1.txt")
        .symlink_metadata()
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(dir
        .join("f2.txt")
        .symlink_metadata()
        .unwrap()
        .file_type()
        .is_symlink());

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

    let output = idlebox_command()
        .args(["readlink", link.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["readlink", "-f", link.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["readlink", "-n", link.to_str().unwrap()])
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
    let output = idlebox_command()
        .args(["uname", "-s"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(
        !stdout.is_empty(),
        "uname -s should output a non-empty sysname"
    );
}

#[test]
#[cfg(unix)]
fn test_uname_all() {
    let output = idlebox_command()
        .args(["uname", "-a"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.split_whitespace().collect();
    assert!(
        parts.len() >= 3,
        "uname -a should output at least 3 fields, got: {:?}",
        parts
    );
}

#[test]
#[cfg(unix)]
fn test_uname_default_is_sysname() {
    let output = idlebox_command()
        .args(["uname"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(
        !stdout.is_empty(),
        "uname should output a non-empty sysname"
    );
}

#[test]
#[cfg(not(unix))]
fn test_uname_sysname() {
    let output = idlebox_command()
        .args(["uname", "-s"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(!stdout.is_empty());
}

#[test]
#[cfg(not(unix))]
fn test_uname_all() {
    let output = idlebox_command()
        .args(["uname", "-a"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.split_whitespace().collect();
    assert!(!parts.is_empty(), "uname -a should output at least 1 field");
}

#[test]
#[cfg(not(unix))]
fn test_uname_default_is_sysname() {
    let output = idlebox_command()
        .args(["uname"])
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

    let output = idlebox_command()
        .args(["test", "-f", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["test", "-d", tmp_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_bracket_numeric_equal() {
    let output = idlebox_command()
        .args(["[", "1", "-eq", "1", "]"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
}

#[test]
fn test_bracket_string_equal() {
    let output = idlebox_command()
        .args(["[", "a", "=", "b", "]"])
        .output()
        .expect("failed to execute process");

    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn test_test_string_zero_length() {
    let output = idlebox_command()
        .args(["test", "-z", ""])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
}

#[test]
fn test_test_string_nonzero_length() {
    let output = idlebox_command()
        .args(["test", "-n", "hello"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
}

#[test]
fn test_test_numeric_comparison() {
    let output = idlebox_command()
        .args(["test", "5", "-gt", "3"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
}

#[test]
fn test_test_logical_and() {
    let output = idlebox_command()
        .args(["test", "1", "-eq", "1", "-a", "2", "-eq", "2"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
}

#[test]
fn test_test_logical_or() {
    let output = idlebox_command()
        .args(["test", "1", "-eq", "2", "-o", "2", "-eq", "2"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
}

#[test]
fn test_test_logical_not() {
    let output = idlebox_command()
        .args(["test", "!", "1", "-eq", "2"])
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

    let output = idlebox_command()
        .args(["test", "-s", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["test", "-L", link.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_expr_addition() {
    let output = idlebox_command()
        .args(["expr", "5", "+", "3"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "8");
}

#[test]
fn test_expr_multiplication() {
    let output = idlebox_command()
        .args(["expr", "10", "*", "2"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "20");
}

#[test]
fn test_expr_length() {
    let output = idlebox_command()
        .args(["expr", "length", "hello"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "5");
}

#[test]
fn test_expr_substring() {
    let output = idlebox_command()
        .args(["expr", "substr", "hello", "2", "3"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ell");
}

#[test]
fn test_expr_comparison() {
    let output = idlebox_command()
        .args(["expr", "5", ">", "3"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
}

#[test]
fn test_expr_logical_or() {
    let output = idlebox_command()
        .args(["expr", "0", "|", "5"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "5");
}

#[test]
fn test_expr_logical_and() {
    let output = idlebox_command()
        .args(["expr", "3", "&", "5"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "3");
}

#[test]
fn test_expr_division_by_zero() {
    let output = idlebox_command()
        .args(["expr", "10", "/", "0"])
        .output()
        .expect("failed to execute process");

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_expr_modulo() {
    let output = idlebox_command()
        .args(["expr", "10", "%", "3"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
}

#[test]
fn test_expr_string_equality() {
    let output = idlebox_command()
        .args(["expr", "hello", "=", "hello"])
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

    let output = idlebox_command()
        .args(["find", tmp_dir.to_str().unwrap(), "-name", "*.rs"])
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

    let output = idlebox_command()
        .args([
            "find",
            tmp_dir.to_str().unwrap(),
            "-type",
            "d",
            "-maxdepth",
            "1",
        ])
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

    let output = idlebox_command()
        .args([
            "find",
            tmp_dir.to_str().unwrap(),
            "-name",
            "*.txt",
            "-maxdepth",
            "1",
        ])
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

    let output = idlebox_command()
        .args(["find", tmp_dir.to_str().unwrap(), "-empty"])
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
    let output = idlebox_command()
        .args(["find"])
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

    let output = idlebox_command()
        .args(["find", tmp_dir.to_str().unwrap(), "-type", "f"])
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

    let output = idlebox_command()
        .args(["find", tmp_dir.to_str().unwrap(), "-type", "l"])
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

    let output = idlebox_command()
        .args(["wc", "-l", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["wc", "-w", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("5"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_wc_stdin() {
    let mut child = idlebox_command()
        .args(["wc", "-l"])
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

    let output = idlebox_command()
        .args(["wc", "-l", f1.to_str().unwrap(), f2.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("total"));
    assert!(stdout.contains("5"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_wc_streams_utf8_across_buffer_boundaries() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_wc_utf8_boundary");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    let content = format!("{}😊\n", "a".repeat(8191));
    fs::write(&file, content).unwrap();

    let output = idlebox_command()
        .args(["wc", "-lwmc", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let counts = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .take(4)
        .map(|value| value.parse::<usize>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(counts, vec![1, 1, 8196, 8193]);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_wc_counts_invalid_utf8_lossily_without_buffering_the_file() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_wc_invalid_utf8");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.bin");
    fs::write(&file, [0xff, b'\n', 0xfe]).unwrap();

    let output = idlebox_command()
        .args(["wc", "-m", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .next(),
        Some("3")
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_sort_basic() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_sort");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "cherry\napple\nbanana\n").unwrap();

    let output = idlebox_command()
        .args(["sort", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["sort", "-n", "-r", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["sort", "-u", file.to_str().unwrap()])
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
    let mut child = idlebox_command()
        .args(["sort"])
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

    let output = idlebox_command()
        .args(["uniq", "-c", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["uniq", "-d", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["uniq", "-u", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["uniq", "-i", "-c", file.to_str().unwrap()])
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
fn test_uniq_streams_to_output_file() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_uniq_output");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let input = tmp_dir.join("input.txt");
    let output = tmp_dir.join("output.txt");
    fs::write(&input, "a\na\nb\n").unwrap();
    fs::write(&output, "old output\n").unwrap();

    let result = idlebox_command()
        .args([
            "uniq",
            "-c",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute process");

    assert!(result.status.success());
    assert!(result.stdout.is_empty());
    let written = fs::read_to_string(output).unwrap();
    assert!(written.contains("2 a"));
    assert!(written.contains("1 b"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_uniq_rejects_hard_link_output_alias_without_truncating_input() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_uniq_hardlink_alias");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let input = tmp_dir.join("input.txt");
    let output = tmp_dir.join("output.txt");
    fs::write(&input, "a\na\nb\n").unwrap();
    fs::hard_link(&input, &output).unwrap();

    let result = idlebox_command()
        .args(["uniq", input.to_str().unwrap(), output.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("different files"));
    assert_eq!(fs::read_to_string(&input).unwrap(), "a\na\nb\n");
    assert_eq!(fs::read_to_string(&output).unwrap(), "a\na\nb\n");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_uniq_rejects_symlink_output_alias_without_truncating_input() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_uniq_symlink_alias");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let input = tmp_dir.join("input.txt");
    let output = tmp_dir.join("output.txt");
    fs::write(&input, "a\na\nb\n").unwrap();
    std::os::unix::fs::symlink(&input, &output).unwrap();

    let result = idlebox_command()
        .args(["uniq", input.to_str().unwrap(), output.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("different files"));
    assert_eq!(fs::read_to_string(&input).unwrap(), "a\na\nb\n");
    assert!(output.symlink_metadata().unwrap().file_type().is_symlink());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_uniq_rejects_extra_operands() {
    let output = idlebox_command()
        .args(["uniq", "first", "second", "third"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("extra operand 'third'"));
}

#[test]
fn test_cut_fields_csv() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cut_f");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.csv");
    fs::write(&file, "name,age,city\nAlice,30,NYC\nBob,25,LA\n").unwrap();

    let output = idlebox_command()
        .args(["cut", "-d", ",", "-f", "1,2", file.to_str().unwrap()])
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

    let output = idlebox_command()
        .args(["cut", "-c", "1-5", file.to_str().unwrap()])
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
    let mut child = idlebox_command()
        .args(["cut", "-d", ":", "-f", "1"])
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
    let mut child = idlebox_command()
        .args(["tr", "a-z", "A-Z"])
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
    let mut child = idlebox_command()
        .args(["tr", "-d", "0-9"])
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
    let mut child = idlebox_command()
        .args(["tr", "-s", " "])
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
    let output = idlebox_command()
        .args(["whoami"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(
        !stdout.is_empty(),
        "whoami should output a non-empty username"
    );
}

#[test]
#[cfg(unix)]
fn test_whoami_matches_id_un() {
    let whoami_output = idlebox_command()
        .args(["whoami"])
        .output()
        .expect("failed to execute process");

    let id_output = idlebox_command()
        .args(["id", "-u", "-n"])
        .output()
        .expect("failed to execute process");

    assert!(whoami_output.status.success());
    assert!(id_output.status.success());
    let whoami_name = String::from_utf8_lossy(&whoami_output.stdout)
        .trim()
        .to_string();
    let id_name = String::from_utf8_lossy(&id_output.stdout)
        .trim()
        .to_string();
    assert_eq!(whoami_name, id_name);
}

#[test]
#[cfg(unix)]
fn test_id_default_format() {
    let output = idlebox_command()
        .args(["id"])
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
    let output = idlebox_command()
        .args(["id", "-u"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(
        stdout.parse::<u32>().is_ok(),
        "id -u should output a numeric UID, got: {}",
        stdout
    );
}

#[test]
#[cfg(unix)]
fn test_id_u_name_flag() {
    let output = idlebox_command()
        .args(["id", "-u", "-n"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(
        !stdout.is_empty(),
        "id -u -n should output a non-empty username"
    );
    assert!(
        stdout.parse::<u32>().is_err(),
        "id -u -n should output a name, not a number"
    );
}

#[test]
#[cfg(unix)]
fn test_id_g_flag() {
    let output = idlebox_command()
        .args(["id", "-g"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(
        stdout.parse::<u32>().is_ok(),
        "id -g should output a numeric GID, got: {}",
        stdout
    );
}

#[test]
#[cfg(unix)]
fn test_id_g_name_flag() {
    let output = idlebox_command()
        .args(["id", "-g", "-n"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(
        !stdout.is_empty(),
        "id -g -n should output a non-empty group name"
    );
}

#[test]
#[cfg(unix)]
fn test_id_g_supplementary_flag() {
    let output = idlebox_command()
        .args(["id", "-G"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        let gids: Vec<&str> = stdout.split_whitespace().collect();
        for gid in &gids {
            assert!(
                gid.parse::<u32>().is_ok(),
                "each group should be numeric, got: {}",
                gid
            );
        }
    }
}

#[test]
#[cfg(unix)]
fn test_id_nonexistent_user() {
    let output = idlebox_command()
        .args(["id", "nonexistent_user_xyz_12345"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no such user"));
}

#[test]
#[cfg(unix)]
fn test_id_combined_flags() {
    let output = idlebox_command()
        .args(["id", "-un"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(!stdout.is_empty(), "id -un should output a username");
    assert!(
        stdout.parse::<u32>().is_err(),
        "id -un should output a name, not a number"
    );
}

#[test]
#[cfg(unix)]
fn test_chown_missing_operand() {
    let output = idlebox_command()
        .args(["chown"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("IdleBox v"));
    assert!(stderr.contains("Usage: chown"));
}

#[test]
#[cfg(unix)]
fn test_chown_invalid_user() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_chown_inv");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("testfile.txt");
    fs::write(&file, "hello").unwrap();

    let output = idlebox_command()
        .args([
            "chown",
            "nonexistent_user_xyz_12345",
            file.to_str().unwrap(),
        ])
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
    let output = idlebox_command()
        .args(["chown", "root", "/tmp/idlebox_nonexistent_file_xyz"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot access"));
}

#[test]
#[cfg(unix)]
fn test_chgrp_missing_operand() {
    let output = idlebox_command()
        .args(["chgrp"])
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

    let output = idlebox_command()
        .args([
            "chgrp",
            "nonexistent_group_xyz_12345",
            file.to_str().unwrap(),
        ])
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
    let output = idlebox_command()
        .args(["chgrp", "0", "/tmp/idlebox_nonexistent_file_xyz"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot access") || stderr.contains("No such file"));
}

#[test]
#[cfg(unix)]
fn test_su_missing_command_argument() {
    let output = idlebox_command()
        .args(["su", "-c"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("requires an argument"));
}

#[test]
#[cfg(unix)]
fn test_su_nonexistent_user() {
    let output = idlebox_command()
        .args(["su", "nonexistent_user_xyz_12345"])
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
    let output = idlebox_command()
        .args(["su", "--help"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("su"));
}

#[test]
fn test_true_and_false_exit_statuses() {
    let success = idlebox_command().arg("true").output().unwrap();
    assert!(success.status.success());
    assert!(success.stdout.is_empty());

    let failure = idlebox_command().arg("false").output().unwrap();
    assert_eq!(failure.status.code(), Some(1));
    assert!(failure.stdout.is_empty());
}

#[test]
fn test_pwd_physical() {
    let output = idlebox_command().args(["pwd", "-P"]).output().unwrap();
    assert_command_success(&output, "printing the current directory");
    let expected = fs::canonicalize(std::env::current_dir().unwrap()).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim_end(),
        expected.to_string_lossy()
    );
}

#[test]
#[cfg(unix)]
fn test_pwd_logical_preserves_valid_symlink_path() {
    let tmp_dir = install_test_dir("pwd_logical");
    let _ = fs::remove_dir_all(&tmp_dir);
    let physical = tmp_dir.join("physical");
    let logical = tmp_dir.join("logical");
    fs::create_dir_all(&physical).unwrap();
    std::os::unix::fs::symlink(&physical, &logical).unwrap();

    let output = idlebox_command()
        .args(["pwd", "-L"])
        .current_dir(&logical)
        .env("PWD", &logical)
        .output()
        .unwrap();
    assert_command_success(&output, "printing a logical current directory");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim_end(),
        logical.to_string_lossy()
    );
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_basename_and_dirname() {
    let basename = idlebox_command()
        .args(["basename", "/usr/local/bin/tool.rs", ".rs"])
        .output()
        .unwrap();
    assert_command_success(&basename, "stripping a base name");
    assert_eq!(basename.stdout, b"tool\n");

    let multiple = idlebox_command()
        .args(["basename", "-s", ".txt", "one.txt", "/tmp/two.txt"])
        .output()
        .unwrap();
    assert_command_success(&multiple, "stripping multiple base names");
    assert_eq!(multiple.stdout, b"one\ntwo\n");

    let dirname = idlebox_command()
        .args(["dirname", "/usr/local/bin/"])
        .output()
        .unwrap();
    assert_command_success(&dirname, "stripping a directory name");
    assert_eq!(dirname.stdout, b"/usr/local\n");
}

#[test]
fn test_realpath_existing_path() {
    let tmp_dir = install_test_dir("realpath");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    let file = tmp_dir.join("file.txt");
    fs::write(&file, b"content").unwrap();

    let output = idlebox_command()
        .args(["realpath", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert_command_success(&output, "canonicalizing a path");
    let expected = fs::canonicalize(&file).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim_end(),
        expected.to_string_lossy()
    );
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_realpath_quiet_continues_after_missing_path() {
    let tmp_dir = install_test_dir("realpath_quiet");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    let existing = tmp_dir.join("existing");
    let missing = tmp_dir.join("missing");
    fs::write(&existing, b"content").unwrap();

    let output = idlebox_command()
        .args([
            "realpath",
            "-q",
            missing.to_str().unwrap(),
            existing.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim_end(),
        fs::canonicalize(&existing).unwrap().to_string_lossy()
    );
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_sleep_zero_and_invalid_interval() {
    let success = idlebox_command()
        .args(["sleep", "0", "0s"])
        .output()
        .unwrap();
    assert_command_success(&success, "sleeping for zero seconds");

    let failure = idlebox_command().args(["sleep", "1week"]).output().unwrap();
    assert_eq!(failure.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&failure.stderr).contains("invalid time interval"));
}

#[test]
fn test_env_prints_modified_environment() {
    let output = idlebox_command()
        .args(["env", "-i", "IDLEBOX_ENV_TEST=works"])
        .output()
        .unwrap();
    assert_command_success(&output, "printing a modified environment");
    assert_eq!(output.stdout, b"IDLEBOX_ENV_TEST=works\n");
}

#[test]
fn test_env_executes_command_with_modified_environment() {
    let output = idlebox_command()
        .args([
            "env",
            "-i",
            "IDLEBOX_ENV_TEST=works",
            env!("CARGO_BIN_EXE_idlebox"),
            "printenv",
            "IDLEBOX_ENV_TEST",
        ])
        .output()
        .unwrap();
    assert_command_success(&output, "executing a command with env");
    assert_eq!(output.stdout, b"works\n");
}

#[test]
fn test_env_does_not_capture_command_help_argument() {
    let output = idlebox_command()
        .args([
            "env",
            env!("CARGO_BIN_EXE_idlebox"),
            "printf",
            "%s",
            "--help",
        ])
        .output()
        .unwrap();
    assert_command_success(&output, "passing command arguments through env");
    assert_eq!(output.stdout, b"--help");
}

#[test]
fn test_printenv_named_variables_and_missing_status() {
    let output = idlebox_command()
        .args(["printenv", "IDLEBOX_PRINTENV_TEST"])
        .env("IDLEBOX_PRINTENV_TEST", "visible")
        .output()
        .unwrap();
    assert_command_success(&output, "printing a named environment variable");
    assert_eq!(output.stdout, b"visible\n");

    let missing = idlebox_command()
        .args(["printenv", "IDLEBOX_VARIABLE_THAT_DOES_NOT_EXIST"])
        .env_remove("IDLEBOX_VARIABLE_THAT_DOES_NOT_EXIST")
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    assert!(missing.stdout.is_empty());
}

#[test]
fn test_printf_formats_and_reuses_format() {
    let output = idlebox_command()
        .args([
            "printf",
            "%s:%04d:%#x:%b\\n",
            "first",
            "7",
            "31",
            "a\\tb",
            "second",
            "8",
            "32",
            "c\\td",
        ])
        .output()
        .unwrap();
    assert_command_success(&output, "formatting arguments");
    assert_eq!(
        output.stdout,
        b"first:0007:0x1f:a\tb\nsecond:0008:0x20:c\td\n"
    );
}

#[test]
fn test_printf_reports_invalid_numbers() {
    let output = idlebox_command()
        .args(["printf", "%d", "not-a-number"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"0");
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected a numeric value"));
}

#[test]
fn test_tee_copies_to_stdout_and_files() {
    let tmp_dir = install_test_dir("tee");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    let file = tmp_dir.join("output.txt");

    let mut child = idlebox_command()
        .args(["tee", file.to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"first line\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_command_success(&output, "copying input with tee");
    assert_eq!(output.stdout, b"first line\n");
    assert_eq!(fs::read(&file).unwrap(), b"first line\n");

    let mut child = idlebox_command()
        .args(["tee", "-a", file.to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"second line\n")
        .unwrap();
    assert!(child.wait().unwrap().success());
    assert_eq!(fs::read(&file).unwrap(), b"first line\nsecond line\n");
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_broken_pipe_exits_without_diagnostic() {
    let tmp_dir = install_test_dir("broken_pipe");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    let file = tmp_dir.join("large.txt");
    fs::write(&file, vec![b'x'; 256 * 1024]).unwrap();

    let mut child = idlebox_command()
        .args(["cat", file.to_str().unwrap()])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let _ = fs::remove_dir_all(&tmp_dir);
}

fn idlebox_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_idlebox"))
}

#[test]
fn test_cat_preserves_exact_bytes() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cat_exact_bytes");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    let file = tmp_dir.join("input.bin");
    let expected = [0xff, 0x00, b'a'];
    fs::write(&file, expected).unwrap();

    let output = idlebox_command()
        .args(["cat", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(output.stdout, expected);
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_head_and_tail_preserve_binary_lines() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_binary_lines");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    let file = tmp_dir.join("input.bin");
    fs::write(&file, [0xff, b'\n', 0xfe]).unwrap();

    let head = idlebox_command()
        .args(["head", "-n", "1", file.to_str().unwrap()])
        .output()
        .unwrap();
    let tail = idlebox_command()
        .args(["tail", "-n", "1", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(head.status.success());
    assert_eq!(head.stdout, [0xff, b'\n']);
    assert!(tail.status.success());
    assert_eq!(tail.stdout, [0xfe]);
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tail_zero_lines_is_empty() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tail_zero");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    let file = tmp_dir.join("input.txt");
    fs::write(&file, "line\n").unwrap();

    let output = idlebox_command()
        .args(["tail", "-n", "0", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_expr_unicode_length_and_substring() {
    let length = idlebox_command()
        .args(["expr", "length", "é😊"])
        .output()
        .unwrap();
    let substring = idlebox_command()
        .args(["expr", "substr", "é😊", "2", "1"])
        .output()
        .unwrap();

    assert!(length.status.success());
    assert_eq!(length.stdout, b"2\n");
    assert!(substring.status.success());
    assert_eq!(String::from_utf8(substring.stdout).unwrap(), "😊\n");
}

#[test]
fn test_expr_overflow_is_reported() {
    let output = idlebox_command()
        .args(["expr", &i64::MAX.to_string(), "+", "1"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("syntax error"));
}

#[test]
fn test_tr_rejects_empty_translation_set() {
    let output = idlebox_command().args(["tr", "a", ""]).output().unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("must not be empty"));
}

#[test]
fn test_cut_preserves_line_without_delimiter() {
    let mut child = idlebox_command()
        .args(["cut", "-d", ",", "-f", "2"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"plain-line\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert_eq!(output.stdout, b"plain-line\n");
}

#[test]
fn test_applet_errors_are_printed() {
    let output = idlebox_command()
        .args(["cat", "--definitely-invalid"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid option"));
}

#[test]
#[cfg(unix)]
fn test_short_h_remains_test_symlink_operator() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_test_h");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    let target = tmp_dir.join("target");
    let link = tmp_dir.join("link");
    fs::write(&target, "content").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let output = idlebox_command()
        .args(["test", "-h", link.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_cp_recursive_preserves_symlink() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cp_symlink_tree");
    let _ = fs::remove_dir_all(&tmp_dir);
    let source = tmp_dir.join("source");
    let outside = tmp_dir.join("outside");
    let destination = tmp_dir.join("destination");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("file"), "outside").unwrap();
    std::os::unix::fs::symlink(&outside, source.join("linked-outside")).unwrap();

    let output = idlebox_command()
        .args([
            "cp",
            "-R",
            source.to_str().unwrap(),
            destination.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(fs::symlink_metadata(destination.join("linked-outside"))
        .unwrap()
        .file_type()
        .is_symlink());
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_cp_rejects_copy_into_itself() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_cp_self");
    let _ = fs::remove_dir_all(&tmp_dir);
    let source = tmp_dir.join("source");
    let destination = source.join("child");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("file"), "content").unwrap();

    let output = idlebox_command()
        .args([
            "cp",
            "-R",
            source.to_str().unwrap(),
            destination.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(!destination.exists());
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_recursive_chmod_does_not_follow_symlink() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_chmod_symlink_tree");
    let _ = fs::remove_dir_all(&tmp_dir);
    let tree = tmp_dir.join("tree");
    let outside = tmp_dir.join("outside");
    fs::create_dir_all(&tree).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let outside_file = outside.join("file");
    fs::write(&outside_file, "content").unwrap();
    fs::set_permissions(&outside_file, fs::Permissions::from_mode(0o640)).unwrap();
    std::os::unix::fs::symlink(&outside, tree.join("linked-outside")).unwrap();

    let output = idlebox_command()
        .args(["chmod", "-R", "700", tree.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(fs::metadata(&outside_file).unwrap().mode() & 0o777, 0o640);
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_du_does_not_follow_directory_symlink() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_du_symlink_tree");
    let _ = fs::remove_dir_all(&tmp_dir);
    let tree = tmp_dir.join("tree");
    let outside = tmp_dir.join("outside");
    fs::create_dir_all(&tree).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("file"), "content").unwrap();
    std::os::unix::fs::symlink(&outside, tree.join("linked-outside")).unwrap();

    let output = idlebox_command()
        .args(["du", tree.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("linked-outside"));
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_rm_symlink_to_directory_without_recursive_flag() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_rm_directory_symlink");
    let _ = fs::remove_dir_all(&tmp_dir);
    let outside = tmp_dir.join("outside");
    let link = tmp_dir.join("link");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("file"), "content").unwrap();
    std::os::unix::fs::symlink(&outside, &link).unwrap();

    let output = idlebox_command()
        .args(["rm", link.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(outside.join("file").exists());
    assert!(link.symlink_metadata().is_err());
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_ls_lists_broken_symlink() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_ls_broken_symlink");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    let link = tmp_dir.join("broken-link");
    std::os::unix::fs::symlink("missing-target", &link).unwrap();

    let output = idlebox_command()
        .args(["ls", "-l", link.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("broken-link -> missing-target"));
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_id_groups_match_system_id() {
    use std::collections::BTreeSet;

    let actual = idlebox_command().args(["id", "-G"]).output().unwrap();
    let expected = Command::new("id").arg("-G").output().unwrap();
    let parse = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .split_whitespace()
            .map(|value| value.parse::<u32>().unwrap())
            .collect::<BTreeSet<_>>()
    };

    assert!(actual.status.success());
    assert!(expected.status.success());
    assert_eq!(parse(&actual.stdout), parse(&expected.stdout));
}

#[test]
#[cfg(target_os = "macos")]
fn test_kill_uses_macos_signal_numbers() {
    let output = idlebox_command().args(["kill", "-l"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("10) SIGBUS"));
    assert!(stdout.contains("30) SIGUSR1"));
}

#[test]
#[cfg(windows)]
fn test_df_does_not_modify_probe_file() {
    let tmp_dir =
        std::env::temp_dir().join(format!("idlebox_test_df_probe_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    let probe = tmp_dir.join(".idlebox_df_probe");
    fs::write(&probe, "keep-me").unwrap();

    let output = idlebox_command()
        .args(["df", tmp_dir.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(fs::read_to_string(&probe).unwrap(), "keep-me");
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_gzip_gunzip_and_zcat_file_workflow() {
    let tmp_dir = archive_test_dir("gzip_files");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    let input = tmp_dir.join("payload.txt");
    let compressed = tmp_dir.join("payload.txt.gz");
    let original = b"archive payload\nwith a second line\n";
    fs::write(&input, original).unwrap();

    let output = idlebox_command()
        .args(["gzip", "--keep", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_command_success(&output, "gzip -k");
    assert!(input.exists());
    assert!(compressed.exists());

    let zcat = idlebox_command()
        .args(["zcat", compressed.to_str().unwrap()])
        .output()
        .unwrap();
    assert_command_success(&zcat, "zcat file");
    assert_eq!(zcat.stdout, original);

    let gzip_decompress = idlebox_command()
        .args([
            "gzip",
            "--decompress",
            "--to-stdout",
            compressed.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_command_success(&gzip_decompress, "gzip -dc");
    assert_eq!(gzip_decompress.stdout, original);

    fs::remove_file(&input).unwrap();
    let gunzip = idlebox_command()
        .args(["gunzip", "--keep", compressed.to_str().unwrap()])
        .output()
        .unwrap();
    assert_command_success(&gunzip, "gunzip -k");
    assert_eq!(fs::read(&input).unwrap(), original);
    assert!(compressed.exists());

    fs::write(&input, b"new payload").unwrap();
    let refused = idlebox_command()
        .args(["gzip", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert_eq!(fs::read(&input).unwrap(), b"new payload");

    let forced = idlebox_command()
        .args(["gzip", "--force", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_command_success(&forced, "gzip -f");
    assert!(!input.exists());

    fs::write(&input, b"stale output").unwrap();
    let forced_gunzip = idlebox_command()
        .args(["gunzip", "-kf", compressed.to_str().unwrap()])
        .output()
        .unwrap();
    assert_command_success(&forced_gunzip, "gunzip -kf");
    assert_eq!(fs::read(&input).unwrap(), b"new payload");
    assert!(compressed.exists());

    let gunzip_stdout = idlebox_command()
        .args(["gunzip", "--to-stdout", compressed.to_str().unwrap()])
        .output()
        .unwrap();
    assert_command_success(&gunzip_stdout, "gunzip -c");
    assert_eq!(gunzip_stdout.stdout, b"new payload");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_gzip_and_zcat_standard_streams() {
    let payload = b"streamed gzip payload\0with binary data\xff";
    let mut gzip = idlebox_command()
        .args(["gzip", "-c"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    gzip.stdin.as_mut().unwrap().write_all(payload).unwrap();
    let compressed = gzip.wait_with_output().unwrap();
    assert_command_success(&compressed, "gzip stdin");
    assert!(compressed.stdout.starts_with(&[0x1f, 0x8b]));

    let mut zcat = idlebox_command()
        .arg("zcat")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    zcat.stdin
        .as_mut()
        .unwrap()
        .write_all(&compressed.stdout)
        .unwrap();
    let decompressed = zcat.wait_with_output().unwrap();
    assert_command_success(&decompressed, "zcat stdin");
    assert_eq!(decompressed.stdout, payload);
}

#[test]
fn test_tar_create_list_extract_and_gzip_modes() {
    let tmp_dir = archive_test_dir("tar_roundtrip");
    let _ = fs::remove_dir_all(&tmp_dir);
    let input = tmp_dir.join("input");
    fs::create_dir_all(input.join("nested")).unwrap();
    fs::write(input.join("root.txt"), b"root file\n").unwrap();
    fs::write(input.join("nested/data.bin"), b"nested\0data\xff").unwrap();
    let archive = tmp_dir.join("bundle.tar");

    let create = idlebox_command()
        .current_dir(&tmp_dir)
        .args(["tar", "-cvf", archive.to_str().unwrap(), "input"])
        .output()
        .unwrap();
    assert_command_success(&create, "tar -cvf");
    assert!(String::from_utf8_lossy(&create.stdout).contains("input"));

    let list = idlebox_command()
        .args(["tar", "-tf", archive.to_str().unwrap()])
        .output()
        .unwrap();
    assert_command_success(&list, "tar -tf");
    let listing = String::from_utf8_lossy(&list.stdout);
    assert!(listing.contains("input/root.txt"));
    assert!(listing.contains("input/nested/data.bin"));

    let long_list = idlebox_command()
        .args(["tar", "--list", "--file", archive.to_str().unwrap()])
        .output()
        .unwrap();
    assert_command_success(&long_list, "tar --list --file");
    assert_eq!(long_list.stdout, list.stdout);

    let extracted = tmp_dir.join("extracted");
    fs::create_dir(&extracted).unwrap();
    let extract = idlebox_command()
        .args([
            "tar",
            "-xvf",
            archive.to_str().unwrap(),
            "-C",
            extracted.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_command_success(&extract, "tar -xvf");
    assert_eq!(
        fs::read(extracted.join("input/nested/data.bin")).unwrap(),
        b"nested\0data\xff"
    );

    let gzip_archive = tmp_dir.join("bundle.tar.gz");
    let create_gzip = idlebox_command()
        .current_dir(&tmp_dir)
        .args(["tar", "-czvf", gzip_archive.to_str().unwrap(), "input"])
        .output()
        .unwrap();
    assert_command_success(&create_gzip, "tar -czvf");

    let list_gzip = idlebox_command()
        .args(["tar", "-tzf", gzip_archive.to_str().unwrap()])
        .output()
        .unwrap();
    assert_command_success(&list_gzip, "tar -tzf");
    assert!(String::from_utf8_lossy(&list_gzip.stdout).contains("input/root.txt"));

    let extracted_gzip = tmp_dir.join("extracted-gzip");
    fs::create_dir(&extracted_gzip).unwrap();
    let extract_gzip = idlebox_command()
        .args([
            "tar",
            "-xzvf",
            gzip_archive.to_str().unwrap(),
            "-C",
            extracted_gzip.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_command_success(&extract_gzip, "tar -xzvf");
    assert_eq!(
        fs::read(extracted_gzip.join("input/root.txt")).unwrap(),
        b"root file\n"
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_unzip_list_extract_and_overwrite() {
    let tmp_dir = archive_test_dir("unzip_roundtrip");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    let archive = tmp_dir.join("fixture.zip");
    write_zip_fixture(
        &archive,
        &[
            ("stored.txt", b"stored entry\n", 0),
            ("nested/deflated.txt", b"deflated entry\n", 8),
        ],
    );

    let list = idlebox_command()
        .args(["unzip", "--list", archive.to_str().unwrap()])
        .output()
        .unwrap();
    assert_command_success(&list, "unzip --list");
    let listing = String::from_utf8_lossy(&list.stdout);
    assert!(listing.contains("stored.txt"));
    assert!(listing.contains("nested/deflated.txt"));

    let output_dir = tmp_dir.join("output");
    let extract = idlebox_command()
        .args([
            "unzip",
            archive.to_str().unwrap(),
            "-d",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_command_success(&extract, "unzip -d");
    assert_eq!(
        fs::read(output_dir.join("stored.txt")).unwrap(),
        b"stored entry\n"
    );
    assert_eq!(
        fs::read(output_dir.join("nested/deflated.txt")).unwrap(),
        b"deflated entry\n"
    );

    fs::write(output_dir.join("stored.txt"), b"preserve me").unwrap();
    let refused = idlebox_command()
        .args([
            "unzip",
            archive.to_str().unwrap(),
            "-d",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert_eq!(
        fs::read(output_dir.join("stored.txt")).unwrap(),
        b"preserve me"
    );

    let overwrite = idlebox_command()
        .args([
            "unzip",
            "--overwrite",
            archive.to_str().unwrap(),
            "-d",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_command_success(&overwrite, "unzip --overwrite");
    assert_eq!(
        fs::read(output_dir.join("stored.txt")).unwrap(),
        b"stored entry\n"
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_unzip_rejects_parent_path_escape() {
    let tmp_dir = archive_test_dir("unzip_escape");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    let archive = tmp_dir.join("escape.zip");
    write_zip_fixture(&archive, &[("../escaped.txt", b"must not escape", 0)]);
    let output_dir = tmp_dir.join("output");

    let output = idlebox_command()
        .args([
            "unzip",
            archive.to_str().unwrap(),
            "-d",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!tmp_dir.join("escaped.txt").exists());

    let _ = fs::remove_dir_all(&tmp_dir);
}

// -- tree ------------------------------------------------------------------

/// Shared shape for the tree tests: 3 directories and 4 visible files, plus one
/// hidden file that only `-a` reveals.
///
/// ```text
/// root/
///   .hidden.txt
///   b.txt
///   a_dir/
///     nested.rs
///     sub/
///       deep.txt
///   z_dir/
///     x.md
/// ```
fn tree_fixture(name: &str) -> PathBuf {
    let tmp_dir = std::env::temp_dir().join(format!("idlebox_test_tree_{}", name));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(tmp_dir.join("a_dir").join("sub")).unwrap();
    fs::create_dir_all(tmp_dir.join("z_dir")).unwrap();
    fs::write(tmp_dir.join(".hidden.txt"), "hidden").unwrap();
    fs::write(tmp_dir.join("b.txt"), "hello").unwrap();
    fs::write(tmp_dir.join("a_dir").join("nested.rs"), "fn main() {}").unwrap();
    fs::write(tmp_dir.join("a_dir").join("sub").join("deep.txt"), "deep").unwrap();
    fs::write(tmp_dir.join("z_dir").join("x.md"), "# md").unwrap();
    tmp_dir
}

fn run_tree(args: &[&str]) -> std::process::Output {
    let mut command = idlebox_command();
    command.arg("tree");
    command.args(args);
    command.output().expect("failed to execute process")
}

/// `tree` run from inside a directory, so the arguments can stay relative.
fn run_tree_in(dir: &Path, args: &[&str]) -> std::process::Output {
    let mut command = idlebox_command();
    command.current_dir(dir);
    command.arg("tree");
    command.args(args);
    command.output().expect("failed to execute process")
}

/// A listing with the root line dropped, so an assertion about which entries
/// were listed cannot be answered by the root path instead.
///
/// The fixtures live under the system temporary directory, which on macOS is
/// `/var/folders/<hash>/<hash>/T` — and "f-old-ers" already contains one of the
/// names the pattern tests search for.
fn tree_entries(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Creates a file whose name is not valid UTF-8, reporting whether the
/// filesystem accepted it. A Unix file name is a byte string, but macOS
/// enforces UTF-8 on APFS and rejects the name outright with `EILSEQ`, so
/// there is nothing for the raw-byte tests to observe there.
#[cfg(unix)]
fn write_non_utf8_named_file(dir: &Path, name: &[u8]) -> bool {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    fs::write(dir.join(OsStr::from_bytes(name)), "x").is_ok()
}

/// The account name and group name the fixtures will be owned by, resolved the
/// same way `tree` resolves them so the metadata columns can be matched exactly.
#[cfg(unix)]
fn tree_expected_owner() -> (String, String) {
    let user = idlebox_command()
        .arg("whoami")
        .output()
        .expect("failed to execute process");
    let group = idlebox_command()
        .args(["id", "-gn"])
        .output()
        .expect("failed to execute process");

    (
        String::from_utf8_lossy(&user.stdout).trim().to_string(),
        String::from_utf8_lossy(&group.stdout).trim().to_string(),
    )
}

#[test]
fn test_tree_basic() {
    let tmp_dir = tree_fixture("basic");

    let output = run_tree(&[tmp_dir.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("├──"));
    assert!(stdout.contains("└──"));
    assert!(stdout.contains("│"));
    assert!(stdout.contains("a_dir"));
    assert!(stdout.contains("nested.rs"));
    assert!(stdout.contains("deep.txt"));
    assert!(stdout.contains("x.md"));
    assert!(!stdout.contains(".hidden.txt"));
    // The root directory counts too, the way upstream's emit_tree() does.
    assert!(stdout.contains("4 directories, 4 files"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_all_shows_hidden() {
    let tmp_dir = tree_fixture("all");

    let hidden = run_tree(&["-a", tmp_dir.to_str().unwrap()]);
    assert!(hidden.status.success());
    let stdout = String::from_utf8_lossy(&hidden.stdout);
    assert!(stdout.contains(".hidden.txt"));
    assert!(stdout.contains("4 directories, 5 files"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_max_depth() {
    let tmp_dir = tree_fixture("depth");

    let output = run_tree(&["-L", "1", tmp_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("a_dir"));
    assert!(stdout.contains("b.txt"));
    assert!(!stdout.contains("nested.rs"));
    assert!(!stdout.contains("deep.txt"));
    assert!(stdout.contains("3 directories, 1 file"));

    // A value-taking option can end a bundle and read the next argument...
    let bundled = run_tree(&["-aL", "1", tmp_dir.to_str().unwrap()]);
    assert!(bundled.status.success());
    assert!(String::from_utf8_lossy(&bundled.stdout).contains(".hidden.txt"));

    // ...or carry the value attached, the way getopt accepts `-L2`.
    let attached = run_tree(&["-L1", tmp_dir.to_str().unwrap()]);
    assert!(attached.status.success());
    let stdout = String::from_utf8_lossy(&attached.stdout);
    assert!(stdout.contains("a_dir"));
    assert!(!stdout.contains("nested.rs"));

    let attached_pattern = run_tree(&["-P*.rs", tmp_dir.to_str().unwrap()]);
    assert!(attached_pattern.status.success());
    let stdout = String::from_utf8_lossy(&attached_pattern.stdout);
    assert!(stdout.contains("nested.rs"));
    assert!(!stdout.contains("b.txt"));

    // An attached value that is not a number is a level error, not a bundle error.
    let non_numeric = run_tree(&["-La", tmp_dir.to_str().unwrap()]);
    assert_eq!(non_numeric.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&non_numeric.stderr).contains("must be greater than 0"));

    let missing_value = run_tree(&["-L"]);
    assert_eq!(missing_value.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&missing_value.stderr).contains("requires an argument"));

    let zero = run_tree(&["-L", "0", tmp_dir.to_str().unwrap()]);
    assert_eq!(zero.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&zero.stderr).contains("must be greater than 0"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_dirs_only() {
    let tmp_dir = tree_fixture("dirsonly");

    let output = run_tree(&["-d", tmp_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("a_dir"));
    assert!(stdout.contains("sub"));
    assert!(stdout.contains("z_dir"));
    assert!(!stdout.contains("b.txt"));
    assert!(!stdout.contains("nested.rs"));
    assert!(stdout.contains("4 directories"));
    assert!(!stdout.contains("files"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_patterns() {
    let tmp_dir = tree_fixture("patterns");

    // -P keeps directories so the tree still has a shape to hang files on.
    let include = run_tree(&["-P", "*.rs", tmp_dir.to_str().unwrap()]);
    assert!(include.status.success());
    let stdout = String::from_utf8_lossy(&include.stdout);
    assert!(stdout.contains("nested.rs"));
    assert!(stdout.contains("a_dir"));
    assert!(!stdout.contains("b.txt"));
    assert!(!stdout.contains("x.md"));
    assert!(stdout.contains("4 directories, 1 file"));

    let exclude = run_tree(&["-I", "z_dir", tmp_dir.to_str().unwrap()]);
    assert!(exclude.status.success());
    let stdout = String::from_utf8_lossy(&exclude.stdout);
    assert!(!stdout.contains("z_dir"));
    assert!(!stdout.contains("x.md"));
    assert!(stdout.contains("b.txt"));
    assert!(stdout.contains("3 directories, 3 files"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_noindent_and_charset() {
    let tmp_dir = tree_fixture("charset");

    let plain = run_tree(&["-i", tmp_dir.to_str().unwrap()]);
    assert!(plain.status.success());
    let stdout = String::from_utf8_lossy(&plain.stdout);
    assert!(!stdout.contains("├"));
    assert!(!stdout.contains("│"));
    // Flush left at *every* depth, not just the first: the point of -i is that
    // `tree -if` produces a list of bare paths something else can consume.
    for line in stdout.lines() {
        assert!(!line.starts_with(' '), "-i must not indent, got {:?}", line);
    }
    assert!(stdout.contains("\nnested.rs\n"));
    assert!(stdout.contains("\ndeep.txt\n"));

    let flat = run_tree(&["-i", "-f", tmp_dir.to_str().unwrap()]);
    assert!(flat.status.success());
    let stdout = String::from_utf8_lossy(&flat.stdout);
    let deep = tmp_dir.join("a_dir").join("sub").join("deep.txt");
    assert!(stdout.contains(&format!("\n{}\n", deep.display())));

    let ascii = run_tree(&["--charset", "ASCII", tmp_dir.to_str().unwrap()]);
    assert!(ascii.status.success());
    let stdout = String::from_utf8_lossy(&ascii.stdout);
    assert!(stdout.contains("|--"));
    assert!(stdout.contains("`--"));
    assert!(!stdout.contains("├"));

    let utf8 = run_tree(&["--charset", "utf-8", tmp_dir.to_str().unwrap()]);
    assert!(utf8.status.success());
    assert!(String::from_utf8_lossy(&utf8.stdout).contains("├──"));

    let bogus = run_tree(&["--charset", "BOGUS", tmp_dir.to_str().unwrap()]);
    assert_eq!(bogus.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bogus.stderr).contains("unsupported charset"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_sorting() {
    let tmp_dir = tree_fixture("sorting");

    let default = run_tree(&["-L", "1", tmp_dir.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&default.stdout);
    let a_dir = stdout.find("a_dir").unwrap();
    let b_txt = stdout.find("b.txt").unwrap();
    let z_dir = stdout.find("z_dir").unwrap();
    assert!(a_dir < b_txt && b_txt < z_dir);

    // --dirsfirst outranks the alphabetical order but not the reverse flag.
    let dirs_first = run_tree(&["-L", "1", "--dirsfirst", tmp_dir.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&dirs_first.stdout);
    assert!(stdout.find("z_dir").unwrap() < stdout.find("b.txt").unwrap());

    let reverse = run_tree(&["-L", "1", "-r", tmp_dir.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&reverse.stdout);
    assert!(stdout.find("z_dir").unwrap() < stdout.find("a_dir").unwrap());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_time_sort() {
    use std::time::{Duration, SystemTime};

    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_timesort");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    // Names sort a < b < c, modification times sort c < b < a.
    for (name, secs) in [("a.txt", 3_000), ("b.txt", 2_000), ("c.txt", 1_000)] {
        let path = tmp_dir.join(name);
        fs::write(&path, name).unwrap();
        let file = fs::File::options().write(true).open(&path).unwrap();
        file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
            .unwrap();
    }

    let output = run_tree(&["-t", tmp_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.find("c.txt").unwrap() < stdout.find("b.txt").unwrap());
    assert!(stdout.find("b.txt").unwrap() < stdout.find("a.txt").unwrap());

    let reversed = run_tree(&["-t", "-r", tmp_dir.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&reversed.stdout);
    assert!(stdout.find("a.txt").unwrap() < stdout.find("b.txt").unwrap());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_classify_and_size() {
    let tmp_dir = tree_fixture("classify");

    let classify = run_tree(&["-F", "-L", "1", tmp_dir.to_str().unwrap()]);
    assert!(classify.status.success());
    let stdout = String::from_utf8_lossy(&classify.stdout);
    assert!(stdout.contains("a_dir/"));
    assert!(stdout.contains("z_dir/"));

    // b.txt holds "hello", so the byte column has to read 5.
    let size = run_tree(&["-s", "-L", "1", tmp_dir.to_str().unwrap()]);
    assert!(size.status.success());
    let stdout = String::from_utf8_lossy(&size.stdout);
    assert!(stdout.contains("5]  b.txt"));

    let human = run_tree(&["-h", "-L", "1", tmp_dir.to_str().unwrap()]);
    assert!(human.status.success());
    assert!(String::from_utf8_lossy(&human.stdout).contains("5]  b.txt"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// Every column `-p -u -g -D` asks for has to actually appear: asserting only
/// the permission prefix would pass even if the other three vanished.
#[test]
#[cfg(unix)]
fn test_tree_metadata_columns() {
    use std::time::{Duration, SystemTime};

    let tmp_dir = tree_fixture("metadata");

    // Two fixed timestamps, one on each side of the six-month cutoff that
    // decides between a clock time and a year.
    let old = tmp_dir.join("old.txt");
    fs::write(&old, "old").unwrap();
    fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH)
        .unwrap();

    let recent = tmp_dir.join("recent.txt");
    fs::write(&recent, "recent").unwrap();
    let recent_at = SystemTime::now() - Duration::from_secs(3600);
    fs::File::options()
        .write(true)
        .open(&recent)
        .unwrap()
        .set_modified(recent_at)
        .unwrap();

    let (user, group) = tree_expected_owner();

    let output = run_tree(&["-p", "-u", "-g", "-D", "-L", "1", tmp_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[drwx"));
    assert!(stdout.contains("[-rw-"));

    // All four columns live in one bracketed group, in permission, user, group,
    // time order.
    let line = stdout
        .lines()
        .find(|line| line.ends_with("old.txt"))
        .unwrap_or_else(|| panic!("no line for old.txt in {}", stdout));
    let columns = line
        .split_once('[')
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(columns, _)| columns)
        .unwrap_or_else(|| panic!("unterminated metadata group: {}", line));
    assert!(
        columns.starts_with(&format!("-rw-r--r-- {:<8} {:<8} ", user, group)),
        "expected permissions, user and group columns in {:?}",
        columns
    );
    // A timestamp older than six months is shown with its year, as `ls -l` and
    // upstream `tree` both do.
    assert!(
        columns.ends_with("Jan  1  1970"),
        "expected a year for an epoch mtime in {:?}",
        columns
    );

    // Something recent keeps the HH:MM form instead.
    let line = stdout
        .lines()
        .find(|line| line.ends_with("recent.txt"))
        .unwrap_or_else(|| panic!("no line for recent.txt in {}", stdout));
    let columns = line
        .split_once('[')
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(columns, _)| columns)
        .unwrap_or_else(|| panic!("unterminated metadata group: {}", line));
    let time = columns.rsplit(' ').next().unwrap_or_default();
    assert!(
        time.len() == 5 && time.as_bytes()[2] == b':',
        "expected a HH:MM time for a recent mtime in {:?}",
        columns
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_json() {
    let tmp_dir = tree_fixture("json");

    let output = run_tree(&["-J", tmp_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with('['));
    assert!(stdout.contains("{\"type\":\"directory\",\"name\":\"a_dir\",\"contents\":["));
    assert!(stdout.contains("{\"type\":\"file\",\"name\":\"nested.rs\"}"));
    assert!(stdout.contains("{\"type\":\"report\",\"directories\":4,\"files\":4}"));

    let with_size = run_tree(&["-J", "-s", tmp_dir.to_str().unwrap()]);
    assert!(String::from_utf8_lossy(&with_size.stdout).contains("\"name\":\"b.txt\",\"size\":5"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_tree_json_escapes_special_names() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_jsonescape");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    fs::write(tmp_dir.join("qu\"ote"), "x").unwrap();
    fs::write(tmp_dir.join("back\\slash"), "x").unwrap();

    let output = run_tree(&["-J", tmp_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""name":"qu\"ote""#));
    assert!(stdout.contains(r#""name":"back\\slash""#));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_xml() {
    let tmp_dir = tree_fixture("xml");

    let output = run_tree(&["-X", tmp_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(stdout.contains("<directory name=\"a_dir\">"));
    assert!(stdout.contains("<file name=\"nested.rs\"></file>"));
    assert!(stdout.contains("<directories>4</directories>"));
    assert!(stdout.contains("<files>4</files>"));
    assert!(stdout.trim_end().ends_with("</tree>"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_tree_xml_escapes_special_names() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_xmlescape");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    fs::write(tmp_dir.join("a&b<c>"), "x").unwrap();

    let output = run_tree(&["-X", tmp_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("name=\"a&amp;b&lt;c&gt;\""));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_html() {
    let tmp_dir = tree_fixture("html");

    let output = run_tree(&["-H", "https://example.com/files", tmp_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<!DOCTYPE html>"));
    assert!(stdout.contains("<html>"));
    assert!(stdout.contains("<a href=\"https://example.com/files/a_dir\">a_dir</a>"));
    assert!(stdout.contains("<a href=\"https://example.com/files/a_dir/nested.rs\">nested.rs</a>"));
    assert!(stdout.contains("<p>4 directories, 4 files</p>"));
    assert!(stdout.trim_end().ends_with("</html>"));

    // Metadata columns reach the HTML output too, with the padding preserved.
    let with_meta = run_tree(&[
        "-s",
        "-H",
        "https://example.com/files",
        tmp_dir.to_str().unwrap(),
    ]);
    assert!(with_meta.status.success());
    let stdout = String::from_utf8_lossy(&with_meta.stdout);
    assert!(stdout.contains("5]&nbsp;&nbsp;<a href="));
    assert!(!stdout.contains("5]  <a href="));

    // The columns are escaped a run at a time rather than a byte at a time, so
    // pin the exact padding: `-p -s` right-aligns the size in eight columns and
    // closes with two spaces, and every one of those has to be an `&nbsp;`.
    let padded = run_tree(&[
        "-p",
        "-s",
        "-H",
        "https://example.com/files",
        tmp_dir.to_str().unwrap(),
    ]);
    assert!(padded.status.success());
    let stdout = String::from_utf8_lossy(&padded.stdout);
    assert!(
        stdout.contains(&format!(
            "{}5]{}<a href=",
            "&nbsp;".repeat(8),
            "&nbsp;".repeat(2)
        )),
        "column padding changed: {}",
        stdout
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// A name that is valid UTF-8 but not ASCII has to survive `-X`/`-H` intact:
/// the escaper decodes to check for characters XML cannot spell, and must not
/// mistake the trailing bytes of a multi-byte character for illegal ones.
#[test]
fn test_tree_keeps_multibyte_names_in_markup() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_multibyte");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    fs::write(tmp_dir.join("café.txt"), "x").unwrap();
    fs::write(tmp_dir.join("日本語.md"), "x").unwrap();

    // Read the names back instead of reusing the literals: macOS stores them
    // decomposed, so `café` comes back as `cafe` plus a combining accent. What
    // matters is that `tree` reproduces whatever the filesystem actually holds.
    let mut stored: Vec<String> = fs::read_dir(&tmp_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    stored.sort();
    assert_eq!(stored.len(), 2, "fixture did not land: {:?}", stored);

    let xml = run_tree(&["-X", tmp_dir.to_str().unwrap()]);
    assert!(xml.status.success());
    let stdout = String::from_utf8_lossy(&xml.stdout);
    for name in &stored {
        assert!(
            stdout.contains(&format!("name=\"{}\"", name)),
            "{} is missing from {}",
            name,
            stdout
        );
    }
    // Nothing in a name was substituted; the only `?` belongs to the `<?xml` declaration.
    assert!(
        !stdout
            .lines()
            .any(|line| line.contains("name=") && line.contains('?')),
        "a valid character was replaced: {}",
        stdout
    );

    let html = run_tree(&["-H", "https://example.com/f", tmp_dir.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&html.stdout);
    for name in &stored {
        assert!(
            stdout.contains(&format!(">{}</a>", name)),
            "{} is missing from {}",
            name,
            stdout
        );
    }

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// Names that are legal on disk but not in a URL have to be percent-encoded, or
/// the `#` truncates the link and the `?` turns the rest into a query string.
#[test]
#[cfg(unix)]
fn test_tree_html_percent_encodes_links() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_htmlurl");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    fs::write(tmp_dir.join("we ird#name?q.txt"), "x").unwrap();
    fs::write(tmp_dir.join("a&b.txt"), "x").unwrap();

    let output = run_tree(&["-H", "https://example.com/f", tmp_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("href=\"https://example.com/f/we%20ird%23name%3Fq.txt\""));
    assert!(stdout.contains("href=\"https://example.com/f/a%26b.txt\""));
    // The visible link text stays readable, HTML-escaped rather than encoded.
    assert!(stdout.contains(">we ird#name?q.txt</a>"));
    assert!(stdout.contains(">a&amp;b.txt</a>"));
    // Nothing that could break out of the href attribute survives.
    assert!(!stdout.contains("href=\"https://example.com/f/we ird"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_output_file() {
    let tmp_dir = tree_fixture("outfile");
    let out_file = tmp_dir.join("tree.txt");

    let output = run_tree(&[
        "-o",
        out_file.to_str().unwrap(),
        "-L",
        "1",
        tmp_dir.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let written = fs::read_to_string(&out_file).unwrap();
    assert!(written.contains("├──"));
    assert!(written.contains("a_dir"));
    assert!(written.contains("directories,"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_noreport() {
    let tmp_dir = tree_fixture("noreport");

    let output = run_tree(&["--noreport", tmp_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("a_dir"));
    assert!(!stdout.contains("directories,"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_nonexistent_path() {
    let missing = std::env::temp_dir().join("idlebox_test_tree_missing");
    let _ = fs::remove_dir_all(&missing);

    let output = run_tree(&[missing.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("tree: "));
}

#[test]
fn test_tree_invalid_option() {
    let output = run_tree(&["-Z"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid option -- 'Z'"));
}

#[test]
#[cfg(unix)]
fn test_tree_symlink_is_shown_but_not_followed() {
    let tmp_dir = tree_fixture("symlink");
    std::os::unix::fs::symlink(tmp_dir.join("a_dir"), tmp_dir.join("dirlink")).unwrap();

    let output = run_tree(&[tmp_dir.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dirlink -> "));
    // Following the link would list nested.rs a second time.
    assert_eq!(stdout.matches("nested.rs").count(), 1);
    // Upstream classifies with stat() rather than lstat(), so a link to a
    // directory counts as a directory even though the walk stops at it.
    assert!(stdout.contains("5 directories, 4 files"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// A root that fails to stat must not leave a dangling comma behind: with
/// `--noreport` nothing follows it to absorb one.
#[test]
fn test_tree_json_stays_valid_when_a_root_fails() {
    let tmp_dir = tree_fixture("jsonfail");
    let missing = std::env::temp_dir().join("idlebox_test_tree_jsonfail_missing");
    let _ = fs::remove_dir_all(&missing);

    let output = run_tree(&[
        "-J",
        "--noreport",
        tmp_dir.to_str().unwrap(),
        missing.to_str().unwrap(),
    ]);

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("a_dir"));
    // The last emitted value must be followed by `]`, never by `,`.
    let compact: String = stdout.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        !compact.contains(",]"),
        "trailing comma in JSON: {}",
        stdout
    );
    assert!(compact.ends_with("]"));

    // The failing root is reported on stderr, not silently dropped.
    assert!(String::from_utf8_lossy(&output.stderr).contains("tree: "));

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// A symlink named on the command line is followed, the way `ls link` lists the
/// directory; symlinks found *inside* the tree are still left unexpanded.
#[test]
#[cfg(unix)]
fn test_tree_follows_root_symlink() {
    let tmp_dir = tree_fixture("rootlink");
    let link = std::env::temp_dir().join("idlebox_test_tree_rootlink_link");
    let _ = fs::remove_file(&link);
    std::os::unix::fs::symlink(tmp_dir.join("a_dir"), &link).unwrap();

    let output = run_tree(&[link.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The contents of a_dir, reached through the link.
    assert!(stdout.contains("nested.rs"));
    assert!(stdout.contains("deep.txt"));
    assert!(stdout.contains("-> "));
    assert!(!stdout.contains("0 directories, 0 files"));

    let _ = fs::remove_file(&link);
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
#[cfg(unix)]
fn test_tree_reports_unreadable_directory() {
    let tmp_dir = tree_fixture("denied");
    let locked = tmp_dir.join("locked");
    fs::create_dir_all(locked.join("inner")).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

    // root ignores the mode bits entirely, so there is nothing to observe. CI
    // runs the musl job as root inside a container; skip rather than fail there.
    if fs::read_dir(&locked).is_ok() {
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        let _ = fs::remove_dir_all(&tmp_dir);
        return;
    }

    let output = run_tree(&[tmp_dir.to_str().unwrap()]);

    // Restore before asserting so a failure still leaves a removable directory.
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("locked [error opening dir]"));
    // Other branches keep being walked.
    assert!(stdout.contains("nested.rs"));
    assert!(stdout.contains("x.md"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_full_path() {
    let tmp_dir = tree_fixture("fullpath");

    let output = run_tree(&["-f", tmp_dir.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let nested = tmp_dir.join("a_dir").join("nested.rs");
    let deep = tmp_dir.join("a_dir").join("sub").join("deep.txt");
    assert!(stdout.contains(&nested.display().to_string()));
    assert!(stdout.contains(&deep.display().to_string()));
    // The connectors stay in place; only the names grow a prefix.
    assert!(stdout.contains("├──"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_color() {
    let tmp_dir = tree_fixture("color");

    // Directories are blue, and the escape has to be closed again.
    let colored = run_tree(&["-C", "-L", "1", tmp_dir.to_str().unwrap()]);
    assert!(colored.status.success());
    let stdout = String::from_utf8_lossy(&colored.stdout);
    assert!(stdout.contains("\x1b[1;34ma_dir\x1b[0m"));
    assert!(stdout.contains("b.txt"));
    assert!(!stdout.contains("\x1b[1;34mb.txt"));

    // -C outranks -n whatever the order, the way upstream documents it
    // ("-n  Turn colorization off always (-C overrides)").
    for args in [["-C", "-n", "-L", "1"], ["-n", "-C", "-L", "1"]] {
        let mut argv = args.to_vec();
        argv.push(tmp_dir.to_str().unwrap());
        let forced = run_tree(&argv);
        assert!(forced.status.success());
        assert!(
            String::from_utf8_lossy(&forced.stdout).contains("\x1b[1;34ma_dir\x1b[0m"),
            "-C must win over -n, got {:?}",
            args
        );
    }

    // -n on its own leaves the default plain output alone.
    let plain = run_tree(&["-n", "-L", "1", tmp_dir.to_str().unwrap()]);
    assert!(plain.status.success());
    assert!(!String::from_utf8_lossy(&plain.stdout).contains("\x1b["));

    let default = run_tree(&["-L", "1", tmp_dir.to_str().unwrap()]);
    assert!(!String::from_utf8_lossy(&default.stdout).contains("\x1b["));

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// Both `-u` and `-g` have to reach the output: asserting only the owner would
/// pass even with the group column missing entirely.
#[test]
#[cfg(unix)]
fn test_tree_user_and_group_columns() {
    let tmp_dir = tree_fixture("owner");
    let (user, group) = tree_expected_owner();

    let output = run_tree(&["-u", "-g", "-L", "1", tmp_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Both columns are present, padded, and inside one bracketed group.
    let expected = format!("[{:<8} {:<8}]  ", user, group);
    assert!(
        stdout.contains(&expected),
        "expected {:?} in {}",
        expected,
        stdout
    );
    for line in stdout.lines().filter(|l| l.contains("b.txt")) {
        assert!(line.contains(']'), "unterminated metadata group: {}", line);
    }

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_human_size_uses_units() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_humansize");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    fs::write(tmp_dir.join("big.bin"), vec![0u8; 3_000_000]).unwrap();
    fs::write(tmp_dir.join("small.bin"), vec![0u8; 2_048]).unwrap();

    let human = run_tree(&["-h", tmp_dir.to_str().unwrap()]);
    assert!(human.status.success());
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(stdout.contains("2.9M]  big.bin"), "got {}", stdout);
    assert!(stdout.contains("2.0K]  small.bin"), "got {}", stdout);

    // -s keeps the raw byte count instead.
    let bytes = run_tree(&["-s", tmp_dir.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&bytes.stdout);
    assert!(stdout.contains("3000000]  big.bin"), "got {}", stdout);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_tree_multiple_roots() {
    let first = tree_fixture("multi_a");
    let second = std::env::temp_dir().join("idlebox_test_tree_multi_b");
    let _ = fs::remove_dir_all(&second);
    fs::create_dir_all(&second).unwrap();
    fs::write(second.join("only.txt"), "x").unwrap();

    let output = run_tree(&[first.to_str().unwrap(), second.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("nested.rs"));
    assert!(stdout.contains("only.txt"));
    // One combined report covering both roots, printed once; each root counts
    // itself, so the two contribute 4 + 1 directories.
    assert!(stdout.contains("5 directories, 5 files"));
    assert_eq!(stdout.matches("directories,").count(), 1);
    // The roots are separated by a blank line.
    assert!(stdout.contains(&format!("\n\n{}\n", second.display())));

    let _ = fs::remove_dir_all(&first);
    let _ = fs::remove_dir_all(&second);
}

#[test]
fn test_tree_output_file_error() {
    let tmp_dir = tree_fixture("outerr");
    // A directory is never a valid destination for -o.
    let output = run_tree(&["-o", tmp_dir.to_str().unwrap(), tmp_dir.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot open output file"));
    // The directory it refused to clobber is still there.
    assert!(tmp_dir.join("b.txt").exists());

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// `-o` must never be able to destroy its own input. The report is staged next
/// to the destination and only published once the walk has finished, so a run
/// that overlaps its input or fails part-way leaves the original alone.
#[test]
fn test_tree_output_file_never_clobbers_input() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_outclobber");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    // Reporting on a file and writing the report over it at the same time.
    let input = tmp_dir.join("input.txt");
    fs::write(&input, "ORIGINAL").unwrap();
    let same = run_tree(&["-o", input.to_str().unwrap(), input.to_str().unwrap()]);
    assert_eq!(same.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&input).unwrap(), "ORIGINAL");
    assert!(String::from_utf8_lossy(&same.stderr).contains("both an input and the output file"));

    // A walk that fails must not have truncated an unrelated destination before
    // it even started.
    let result = tmp_dir.join("result.txt");
    fs::write(&result, "PREVIOUS").unwrap();
    let missing = tmp_dir.join("does-not-exist");
    let failed = run_tree(&["-o", result.to_str().unwrap(), missing.to_str().unwrap()]);
    assert_eq!(failed.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("tree: "));
    // The report was still produced, but only after the walk completed.
    let written = fs::read_to_string(&result).unwrap();
    assert!(!written.contains("PREVIOUS"));
    assert!(written.contains("directories,"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// The staging file `-o` writes through lives next to the destination, so it
/// must not turn up in the tree it is recording — not even under `-a`.
#[test]
fn test_tree_output_file_excludes_its_own_staging_file() {
    let tmp_dir = tree_fixture("outstaged");
    let out_file = tmp_dir.join("tree.txt");

    let output = run_tree(&[
        "-a",
        "-o",
        out_file.to_str().unwrap(),
        tmp_dir.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let written = fs::read_to_string(&out_file).unwrap();
    assert!(
        !written.contains("idlebox-"),
        "staging file listed: {}",
        written
    );
    assert!(written.contains(".hidden.txt"));
    // Nothing is left behind next to the destination.
    let leftovers: Vec<_> = fs::read_dir(&tmp_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("idlebox-"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "staging file left behind: {:?}",
        leftovers
    );

    // The same thing again with everything relative, which is how a shell
    // usually spells it. `read_dir(".")` hands back `./x` while the staging
    // path is a bare `x`, so anything comparing the two as text lists the
    // scratch file it is in the middle of writing.
    let bare = run_tree_in(&tmp_dir, &["-a", "-o", "bare.txt", "."]);
    assert!(bare.status.success());
    let written = fs::read_to_string(tmp_dir.join("bare.txt")).unwrap();
    assert!(
        !written.contains("idlebox-"),
        "staging file listed for a bare relative -o: {}",
        written
    );
    assert!(written.contains(".hidden.txt"));

    // ...and with the destination one directory down from the walk root.
    let nested = run_tree_in(&tmp_dir, &["-a", "-o", "z_dir/nested.txt", "."]);
    assert!(nested.status.success());
    let written = fs::read_to_string(tmp_dir.join("z_dir").join("nested.txt")).unwrap();
    assert!(
        !written.contains("idlebox-"),
        "staging file listed for a nested relative -o: {}",
        written
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// `-J`/`-X` classify with upstream's `ftype[]` table, so a symlink is a `link`
/// carrying its target and a socket is a `socket` — not another `file`.
#[test]
#[cfg(unix)]
fn test_tree_serializes_file_types() {
    use std::os::unix::net::UnixListener;

    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_types");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    fs::write(tmp_dir.join("plain.txt"), "x").unwrap();
    std::os::unix::fs::symlink("plain.txt", tmp_dir.join("alias")).unwrap();
    let _listener = UnixListener::bind(tmp_dir.join("sock")).unwrap();

    let json = run_tree(&["-J", tmp_dir.to_str().unwrap()]);
    assert!(json.status.success());
    let stdout = String::from_utf8_lossy(&json.stdout);
    assert!(
        stdout.contains(r#"{"type":"link","name":"alias","target":"plain.txt"}"#),
        "got {}",
        stdout
    );
    assert!(
        stdout.contains(r#"{"type":"socket","name":"sock"}"#),
        "got {}",
        stdout
    );
    assert!(stdout.contains(r#"{"type":"file","name":"plain.txt"}"#));

    let xml = run_tree(&["-X", tmp_dir.to_str().unwrap()]);
    assert!(xml.status.success());
    let stdout = String::from_utf8_lossy(&xml.stdout);
    assert!(
        stdout.contains(r#"<link name="alias" target="plain.txt"></link>"#),
        "got {}",
        stdout
    );
    assert!(
        stdout.contains(r#"<socket name="sock"></socket>"#),
        "got {}",
        stdout
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// Upstream's `-p` schema is a numeric `mode` *plus* a symbolic `prot`; the
/// symbolic string alone is not what a consumer parsing `mode` expects.
#[test]
#[cfg(unix)]
fn test_tree_machine_permission_schema() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_modes");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    let file = tmp_dir.join("script.sh");
    fs::write(&file, "#!/bin/sh\n").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o750)).unwrap();

    let json = run_tree(&["-J", "-p", tmp_dir.to_str().unwrap()]);
    assert!(json.status.success());
    let stdout = String::from_utf8_lossy(&json.stdout);
    assert!(
        stdout.contains(r#""name":"script.sh","mode":"0750","prot":"-rwxr-x---""#),
        "got {}",
        stdout
    );

    let xml = run_tree(&["-X", "-p", tmp_dir.to_str().unwrap()]);
    assert!(xml.status.success());
    let stdout = String::from_utf8_lossy(&xml.stdout);
    assert!(
        stdout.contains(r#"name="script.sh" mode="0750" prot="-rwxr-x---""#),
        "got {}",
        stdout
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// `-h` reaches the JSON size as a quoted string, the way upstream's
/// `json_fillinfo()` writes it. XML deliberately keeps raw bytes: upstream's
/// `xml_fillinfo()` has no human-readable branch and consumers parse a number.
#[test]
fn test_tree_machine_size_modes() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_machinesize");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    fs::write(tmp_dir.join("small.bin"), vec![0u8; 2_048]).unwrap();

    let human = run_tree(&["-J", "-h", tmp_dir.to_str().unwrap()]);
    assert!(human.status.success());
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(
        stdout.contains(r#""name":"small.bin","size":"2.0K""#),
        "got {}",
        stdout
    );

    let bytes = run_tree(&["-J", "-s", tmp_dir.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&bytes.stdout);
    assert!(
        stdout.contains(r#""name":"small.bin","size":2048"#),
        "got {}",
        stdout
    );

    let xml = run_tree(&["-X", "-h", tmp_dir.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&xml.stdout);
    assert!(
        stdout.contains(r#"name="small.bin" size="2048""#),
        "got {}",
        stdout
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// `-d` drops the file total from the machine-readable reports too, matching the
/// text report and upstream's `json_report()`/`xml_report()`.
#[test]
fn test_tree_machine_report_omits_files_with_dirs_only() {
    let tmp_dir = tree_fixture("machinereport");

    let json = run_tree(&["-J", "-d", tmp_dir.to_str().unwrap()]);
    assert!(json.status.success());
    let stdout = String::from_utf8_lossy(&json.stdout);
    assert!(
        stdout.contains(r#"{"type":"report","directories":4}"#),
        "got {}",
        stdout
    );

    let xml = run_tree(&["-X", "-d", tmp_dir.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&xml.stdout);
    assert!(stdout.contains("<directories>4</directories>"));
    assert!(!stdout.contains("<files>"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// A Unix file name is a byte string, not text. Converting it early would swap
/// the bytes for a replacement character, make different files print
/// identically and point the HTML links at paths that do not exist.
#[test]
#[cfg(unix)]
fn test_tree_preserves_non_utf8_names() {
    let raw: &[u8] = &[b'b', b'a', b'd', 0xff];
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_rawbytes");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    if !write_non_utf8_named_file(&tmp_dir, raw) {
        // The filesystem enforces UTF-8 names, so it cannot hold the case this
        // test is about. macOS is the one in CI that does.
        let _ = fs::remove_dir_all(&tmp_dir);
        return;
    }

    let text = run_tree(&[tmp_dir.to_str().unwrap()]);
    assert!(text.status.success());
    assert!(
        text.stdout.windows(raw.len()).any(|window| window == raw),
        "raw bytes did not survive: {:?}",
        String::from_utf8_lossy(&text.stdout)
    );

    let html = run_tree(&["-H", "https://example.com/f", tmp_dir.to_str().unwrap()]);
    assert!(html.status.success());
    let stdout = String::from_utf8_lossy(&html.stdout);
    assert!(
        stdout.contains("href=\"https://example.com/f/bad%FF\""),
        "got {}",
        stdout
    );
    assert!(!stdout.contains("%EF%BF%BD"));

    let json = run_tree(&["-J", tmp_dir.to_str().unwrap()]);
    assert!(json.stdout.windows(raw.len()).any(|window| window == raw));

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// XML 1.0 cannot spell a C0 control character at all, so a name holding one
/// would otherwise produce a document no parser will read.
#[test]
#[cfg(unix)]
fn test_tree_xml_neutralizes_control_characters() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let raw: &[u8] = &[b'c', b't', b'l', 1, b'n', b'a', b'm', b'e'];
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_xmlctl");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    fs::write(tmp_dir.join(OsStr::from_bytes(raw)), "x").unwrap();

    let output = run_tree(&["-X", tmp_dir.to_str().unwrap()]);
    assert!(output.status.success());
    assert!(
        !output.stdout.contains(&1u8),
        "raw control byte reached the XML"
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("name=\"ctl?name\""));

    // JSON can spell it, so there the byte survives as an escape.
    let json = run_tree(&["-J", tmp_dir.to_str().unwrap()]);
    assert!(String::from_utf8_lossy(&json.stdout).contains(r#""name":"ctl\u0001name""#));

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// A document declaring `encoding="UTF-8"` cannot carry a byte that is not
/// valid UTF-8 either, so `-X` has to neutralize those the same way it does the
/// control characters. `-J` deliberately does not: JSON keeps the raw bytes so
/// two differently-named files stay distinguishable.
#[test]
#[cfg(unix)]
fn test_tree_xml_neutralizes_bytes_that_are_not_utf8() {
    let raw: &[u8] = &[b'b', b'a', b'd', 0xff];
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_xmlrawbytes");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    if !write_non_utf8_named_file(&tmp_dir, raw) {
        let _ = fs::remove_dir_all(&tmp_dir);
        return;
    }

    let xml = run_tree(&["-X", tmp_dir.to_str().unwrap()]);
    assert!(xml.status.success());
    assert!(
        std::str::from_utf8(&xml.stdout).is_ok(),
        "XML holds bytes no parser will accept"
    );
    assert!(String::from_utf8_lossy(&xml.stdout).contains("name=\"bad?\""));

    // HTML shares the escaper, so the link text is neutralized too — but the
    // href still points at the real file, because it is percent-encoded.
    let html = run_tree(&["-H", "https://example.com/f", tmp_dir.to_str().unwrap()]);
    assert!(std::str::from_utf8(&html.stdout).is_ok());
    let stdout = String::from_utf8_lossy(&html.stdout);
    assert!(stdout.contains("bad%FF"), "got {}", stdout);
    assert!(stdout.contains(">bad?</a>"), "got {}", stdout);

    // The raw byte still reaches `-J` untouched.
    let json = run_tree(&["-J", tmp_dir.to_str().unwrap()]);
    assert!(json.stdout.windows(raw.len()).any(|window| window == raw));

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// Upstream's `json_listdir()` leaves the `contents` key off a directory with
/// nothing in it rather than writing an empty array, so a consumer can tell an
/// empty directory from a file by the type alone.
#[test]
fn test_tree_json_omits_contents_for_empty_directory() {
    let tmp_dir = tree_fixture("jsonempty");
    fs::create_dir(tmp_dir.join("empty_dir")).unwrap();

    let output = run_tree(&["-J", tmp_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("{\"type\":\"directory\",\"name\":\"empty_dir\"}"),
        "got {}",
        stdout
    );
    // A directory that does hold something still carries the key.
    assert!(stdout.contains("{\"type\":\"directory\",\"name\":\"a_dir\",\"contents\":["));

    // `-d` empties `sub`, which holds only a file, so that drops the key too.
    let dirs_only = run_tree(&["-J", "-d", tmp_dir.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&dirs_only.stdout);
    assert!(
        stdout.contains("{\"type\":\"directory\",\"name\":\"sub\"}"),
        "got {}",
        stdout
    );

    // An empty root has nothing to hang a `contents` on either. Match the JSON
    // key rather than the bare word, which the root path could also spell.
    let root = run_tree(&["-J", tmp_dir.join("empty_dir").to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&root.stdout);
    assert!(!stdout.contains("\"contents\""), "got {}", stdout);

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// A pattern full of `*` used to be matched by trying every split for every
/// star, which is exponential: this one ran for half a minute against ordinary
/// file names before the scan was made greedy.
#[test]
fn test_tree_pattern_does_not_hang() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_patternhang");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    fs::write(tmp_dir.join("a".repeat(60)), "x").unwrap();

    let output = run_tree(&[
        "-a",
        "-P",
        "*?*?*?*?*?*?*?*?*?*?*?*?*?*?*?*Q",
        tmp_dir.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let entries = tree_entries(&output);
    assert!(!entries.contains("aaaa"), "got {}", entries);
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 directory, 0 files"),
        "got {}",
        entries
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// Repeating `-P`/`-I` adds to the pattern set instead of replacing it, and the
/// patterns take the alternation and character classes real `tree` accepts.
#[test]
fn test_tree_pattern_sets_and_syntax() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_patternset");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();
    for name in ["old", "new", "other", "a.rs", "b.md", "c.txt"] {
        fs::write(tmp_dir.join(name), "x").unwrap();
    }

    // Repeated -P keeps both patterns instead of letting the last one win.
    let repeated = run_tree(&["-P", "old", "-P", "new", tmp_dir.to_str().unwrap()]);
    assert!(repeated.status.success());
    let entries = tree_entries(&repeated);
    assert!(entries.contains("old"), "got {}", entries);
    assert!(entries.contains("new"), "got {}", entries);
    assert!(!entries.contains("other"));

    // A single pattern can hold the alternation instead.
    let alternation = run_tree(&["-P", "old|new", tmp_dir.to_str().unwrap()]);
    let entries = tree_entries(&alternation);
    assert!(
        entries.contains("old") && entries.contains("new"),
        "got {}",
        entries
    );
    assert!(!entries.contains("other"));

    // Character classes, ranges and negation.
    let class = run_tree(&["-P", "[ab].*", tmp_dir.to_str().unwrap()]);
    let entries = tree_entries(&class);
    assert!(
        entries.contains("a.rs") && entries.contains("b.md"),
        "got {}",
        entries
    );
    assert!(!entries.contains("c.txt"));

    let negated = run_tree(&["-P", "[^ab]*", tmp_dir.to_str().unwrap()]);
    let entries = tree_entries(&negated);
    assert!(
        entries.contains("c.txt") && entries.contains("old"),
        "got {}",
        entries
    );
    assert!(!entries.contains("a.rs"));

    // Repeated -I removes both, and -I takes the same syntax.
    let excluded = run_tree(&["-I", "old", "-I", "*.md", tmp_dir.to_str().unwrap()]);
    let entries = tree_entries(&excluded);
    assert!(!entries.contains("old"), "got {}", entries);
    assert!(!entries.contains("b.md"));
    assert!(entries.contains("new") && entries.contains("a.rs"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// A link to a directory is classified through the link, the way upstream's
/// `getinfo()` does: `-d` keeps it and `--dirsfirst` sorts it with the
/// directories, even though the walk never descends into it.
#[test]
#[cfg(unix)]
fn test_tree_symlink_to_directory_counts_as_directory() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_tree_dirlink");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(tmp_dir.join("real")).unwrap();
    fs::write(tmp_dir.join("real").join("inner.txt"), "x").unwrap();
    fs::write(tmp_dir.join("zfile.txt"), "x").unwrap();
    std::os::unix::fs::symlink(tmp_dir.join("real"), tmp_dir.join("alink")).unwrap();

    // -d keeps the link but drops the plain file.
    let dirs_only = run_tree(&["-d", tmp_dir.to_str().unwrap()]);
    assert!(dirs_only.status.success());
    let stdout = String::from_utf8_lossy(&dirs_only.stdout);
    assert!(stdout.contains("alink"), "got {}", stdout);
    assert!(!stdout.contains("zfile.txt"));

    // --dirsfirst groups it with the directories rather than the files.
    let dirs_first = run_tree(&["--dirsfirst", "-r", tmp_dir.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&dirs_first.stdout);
    assert!(stdout.find("alink").unwrap() < stdout.find("zfile.txt").unwrap());

    // -P still filters it, because upstream decides that exemption on lstat.
    let filtered = run_tree(&["-P", "zfile*", tmp_dir.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&filtered.stdout);
    assert!(!stdout.contains("alink"), "got {}", stdout);
    assert!(stdout.contains("real"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// A long option accepts its value attached, the way upstream's `long_arg()`
/// handles every `--option=VALUE`.
#[test]
fn test_tree_charset_attached_value() {
    let tmp_dir = tree_fixture("charseteq");

    let ascii = run_tree(&["--charset=ASCII", tmp_dir.to_str().unwrap()]);
    assert!(ascii.status.success());
    let stdout = String::from_utf8_lossy(&ascii.stdout);
    assert!(stdout.contains("|--"));
    assert!(!stdout.contains("├"));

    let utf8 = run_tree(&["--charset=utf-8", tmp_dir.to_str().unwrap()]);
    assert!(utf8.status.success());
    assert!(String::from_utf8_lossy(&utf8.stdout).contains("├──"));

    let empty = run_tree(&["--charset=", tmp_dir.to_str().unwrap()]);
    assert_eq!(empty.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&empty.stderr).contains("requires an argument"));

    let bogus = run_tree(&["--charset=BOGUS", tmp_dir.to_str().unwrap()]);
    assert_eq!(bogus.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bogus.stderr).contains("unsupported charset"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_grep_parallel_matches_single_thread() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_grep_parallel");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    for i in 0..10 {
        let file = tmp_dir.join(format!("file{}.txt", i));
        fs::write(&file, format!("line1\napple{}\nline3\nbanana\n", i)).unwrap();
    }

    let files: Vec<String> = (0..10)
        .map(|i| {
            tmp_dir
                .join(format!("file{}.txt", i))
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect();
    let mut args = vec!["grep".to_string(), "apple".to_string()];
    args.extend(files.iter().cloned());

    let output_single = idlebox_command()
        .args(&args)
        .args(["-j", "1"])
        .output()
        .expect("failed to execute process");

    let output_parallel = idlebox_command()
        .args(&args)
        .args(["-j", "4"])
        .output()
        .expect("failed to execute process");

    assert!(output_single.status.success());
    assert!(output_parallel.status.success());

    let stdout_single = String::from_utf8_lossy(&output_single.stdout);
    let stdout_parallel = String::from_utf8_lossy(&output_parallel.stdout);

    let mut lines_single: Vec<&str> = stdout_single.trim().lines().collect();
    let mut lines_parallel: Vec<&str> = stdout_parallel.trim().lines().collect();
    lines_single.sort();
    lines_parallel.sort();

    assert_eq!(lines_single, lines_parallel);
    assert_eq!(lines_single.len(), 10);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_grep_parallel_count_mode() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_grep_parallel_count");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    for i in 0..5 {
        let file = tmp_dir.join(format!("file{}.txt", i));
        fs::write(&file, "apple\nbanana\napple\ncherry\n").unwrap();
    }

    let files: Vec<String> = (0..5)
        .map(|i| {
            tmp_dir
                .join(format!("file{}.txt", i))
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect();
    let mut args = vec!["grep".to_string(), "-c".to_string(), "apple".to_string()];
    args.extend(files.iter().cloned());

    let output_single = idlebox_command()
        .args(&args)
        .args(["-j", "1"])
        .output()
        .expect("failed to execute process");

    let output_parallel = idlebox_command()
        .args(&args)
        .args(["-j", "4"])
        .output()
        .expect("failed to execute process");

    assert!(output_single.status.success());
    assert!(output_parallel.status.success());

    let stdout_single = String::from_utf8_lossy(&output_single.stdout);
    let stdout_parallel = String::from_utf8_lossy(&output_parallel.stdout);

    let mut lines_single: Vec<&str> = stdout_single.trim().lines().collect();
    let mut lines_parallel: Vec<&str> = stdout_parallel.trim().lines().collect();
    lines_single.sort();
    lines_parallel.sort();

    assert_eq!(lines_single, lines_parallel);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_find_parallel_matches_single_thread() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_find_parallel");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    for i in 0..5 {
        let subdir = tmp_dir.join(format!("dir{}", i));
        fs::create_dir_all(&subdir).unwrap();
        for j in 0..5 {
            let file = subdir.join(format!("file{}.rs", j));
            fs::write(&file, "code").unwrap();
            let file = subdir.join(format!("file{}.txt", j));
            fs::write(&file, "text").unwrap();
        }
    }

    let output_single = idlebox_command()
        .args([
            "find",
            tmp_dir.to_str().unwrap(),
            "-name",
            "*.rs",
            "-j",
            "1",
        ])
        .output()
        .expect("failed to execute process");

    let output_parallel = idlebox_command()
        .args([
            "find",
            tmp_dir.to_str().unwrap(),
            "-name",
            "*.rs",
            "-j",
            "4",
        ])
        .output()
        .expect("failed to execute process");

    assert!(output_single.status.success());
    assert!(output_parallel.status.success());

    let stdout_single = String::from_utf8_lossy(&output_single.stdout);
    let stdout_parallel = String::from_utf8_lossy(&output_parallel.stdout);

    let mut lines_single: Vec<&str> = stdout_single.trim().lines().collect();
    let mut lines_parallel: Vec<&str> = stdout_parallel.trim().lines().collect();
    lines_single.sort();
    lines_parallel.sort();

    assert_eq!(lines_single, lines_parallel);
    assert_eq!(lines_single.len(), 25);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_find_parallel_with_type_filter() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_find_parallel_type");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    for i in 0..3 {
        let subdir = tmp_dir.join(format!("dir{}", i));
        fs::create_dir_all(&subdir).unwrap();
        for j in 0..3 {
            let file = subdir.join(format!("file{}.txt", j));
            fs::write(&file, "content").unwrap();
        }
    }

    let output_single = idlebox_command()
        .args(["find", tmp_dir.to_str().unwrap(), "-type", "f", "-j", "1"])
        .output()
        .expect("failed to execute process");

    let output_parallel = idlebox_command()
        .args(["find", tmp_dir.to_str().unwrap(), "-type", "f", "-j", "4"])
        .output()
        .expect("failed to execute process");

    assert!(output_single.status.success());
    assert!(output_parallel.status.success());

    let stdout_single = String::from_utf8_lossy(&output_single.stdout);
    let stdout_parallel = String::from_utf8_lossy(&output_parallel.stdout);

    let mut lines_single: Vec<&str> = stdout_single.trim().lines().collect();
    let mut lines_parallel: Vec<&str> = stdout_parallel.trim().lines().collect();
    lines_single.sort();
    lines_parallel.sort();

    assert_eq!(lines_single, lines_parallel);
    assert_eq!(lines_single.len(), 9);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_wc_parallel_matches_single_thread() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_wc_parallel");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    for i in 0..8 {
        let file = tmp_dir.join(format!("file{}.txt", i));
        let content = "line1 word1\nline2 word2 word3\nline3\n";
        fs::write(&file, content).unwrap();
    }

    let files: Vec<String> = (0..8)
        .map(|i| {
            tmp_dir
                .join(format!("file{}.txt", i))
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect();
    let mut args = vec!["wc".to_string(), "-l".to_string()];
    args.extend(files.iter().cloned());

    let output_single = idlebox_command()
        .args(&args)
        .args(["-j", "1"])
        .output()
        .expect("failed to execute process");

    let output_parallel = idlebox_command()
        .args(&args)
        .args(["-j", "4"])
        .output()
        .expect("failed to execute process");

    assert!(output_single.status.success());
    assert!(output_parallel.status.success());

    let stdout_single = String::from_utf8_lossy(&output_single.stdout);
    let stdout_parallel = String::from_utf8_lossy(&output_parallel.stdout);

    let mut lines_single: Vec<&str> = stdout_single.trim().lines().collect();
    let mut lines_parallel: Vec<&str> = stdout_parallel.trim().lines().collect();
    lines_single.sort();
    lines_parallel.sort();

    assert_eq!(lines_single, lines_parallel);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_wc_parallel_all_counts() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_wc_parallel_all");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    for i in 0..6 {
        let file = tmp_dir.join(format!("file{}.txt", i));
        let content = "hello world\nfoo bar baz\ntest\n";
        fs::write(&file, content).unwrap();
    }

    let files: Vec<String> = (0..6)
        .map(|i| {
            tmp_dir
                .join(format!("file{}.txt", i))
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect();
    let mut args = vec!["wc".to_string()];
    args.extend(files.iter().cloned());

    let output_single = idlebox_command()
        .args(&args)
        .args(["-j", "1"])
        .output()
        .expect("failed to execute process");

    let output_parallel = idlebox_command()
        .args(&args)
        .args(["-j", "8"])
        .output()
        .expect("failed to execute process");

    assert!(output_single.status.success());
    assert!(output_parallel.status.success());

    let stdout_single = String::from_utf8_lossy(&output_single.stdout);
    let stdout_parallel = String::from_utf8_lossy(&output_parallel.stdout);

    let mut lines_single: Vec<&str> = stdout_single.trim().lines().collect();
    let mut lines_parallel: Vec<&str> = stdout_parallel.trim().lines().collect();
    lines_single.sort();
    lines_parallel.sort();

    assert_eq!(lines_single, lines_parallel);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_grep_parallel_default_threads() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_grep_parallel_default");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    for i in 0..5 {
        let file = tmp_dir.join(format!("file{}.txt", i));
        fs::write(&file, "apple\nbanana\napple\n").unwrap();
    }

    let files: Vec<String> = (0..5)
        .map(|i| {
            tmp_dir
                .join(format!("file{}.txt", i))
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect();
    let mut args = vec!["grep".to_string(), "apple".to_string()];
    args.extend(files.iter().cloned());

    let output = idlebox_command()
        .args(&args)
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 10);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_find_parallel_default_threads() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_find_parallel_default");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    for i in 0..3 {
        let subdir = tmp_dir.join(format!("dir{}", i));
        fs::create_dir_all(&subdir).unwrap();
        for j in 0..3 {
            let file = subdir.join(format!("file{}.txt", j));
            fs::write(&file, "content").unwrap();
        }
    }

    let output = idlebox_command()
        .args(["find", tmp_dir.to_str().unwrap(), "-type", "f"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 9);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_wc_parallel_default_threads() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_wc_parallel_default");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    for i in 0..4 {
        let file = tmp_dir.join(format!("file{}.txt", i));
        fs::write(&file, "line1\nline2\nline3\n").unwrap();
    }

    let files: Vec<String> = (0..4)
        .map(|i| {
            tmp_dir
                .join(format!("file{}.txt", i))
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect();
    let mut args = vec!["wc".to_string(), "-l".to_string()];
    args.extend(files.iter().cloned());

    let output = idlebox_command()
        .args(&args)
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("total"));
    assert!(stdout.contains("12"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_grep_parallel_with_ignore_case() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_grep_parallel_icase");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    for i in 0..5 {
        let file = tmp_dir.join(format!("file{}.txt", i));
        fs::write(&file, "Error\nerror\nERROR\nwarning\n").unwrap();
    }

    let files: Vec<String> = (0..5)
        .map(|i| {
            tmp_dir
                .join(format!("file{}.txt", i))
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect();
    let mut args = vec!["grep".to_string(), "-i".to_string(), "error".to_string()];
    args.extend(files.iter().cloned());

    let output_single = idlebox_command()
        .args(&args)
        .args(["-j", "1"])
        .output()
        .expect("failed to execute process");

    let output_parallel = idlebox_command()
        .args(&args)
        .args(["-j", "4"])
        .output()
        .expect("failed to execute process");

    assert!(output_single.status.success());
    assert!(output_parallel.status.success());

    let stdout_single = String::from_utf8_lossy(&output_single.stdout);
    let stdout_parallel = String::from_utf8_lossy(&output_parallel.stdout);

    let mut lines_single: Vec<&str> = stdout_single.trim().lines().collect();
    let mut lines_parallel: Vec<&str> = stdout_parallel.trim().lines().collect();
    lines_single.sort();
    lines_parallel.sort();

    assert_eq!(lines_single, lines_parallel);
    assert_eq!(lines_single.len(), 15); // 3 matches per file * 5 files

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_grep_invalid_thread_count_zero() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_grep_j0");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "test\n").unwrap();

    let output = idlebox_command()
        .args(["grep", "-j", "0", "test", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid thread count"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_grep_missing_thread_count() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_grep_j_missing");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "test\n").unwrap();

    let output = idlebox_command()
        .args(["grep", "-j", "test", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid thread count"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_find_invalid_thread_count_zero() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_find_j0");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let output = idlebox_command()
        .args(["find", tmp_dir.to_str().unwrap(), "-j", "0"])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid thread count"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_wc_invalid_thread_count_zero() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_wc_j0");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let file = tmp_dir.join("input.txt");
    fs::write(&file, "test\n").unwrap();

    let output = idlebox_command()
        .args(["wc", "-j", "0", file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid thread count"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_parallel_large_thread_count() {
    let tmp_dir = std::env::temp_dir().join("idlebox_test_parallel_large_j");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    for i in 0..3 {
        let file = tmp_dir.join(format!("file{}.txt", i));
        fs::write(&file, "apple\nbanana\n").unwrap();
    }

    let files: Vec<String> = (0..3)
        .map(|i| {
            tmp_dir
                .join(format!("file{}.txt", i))
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect();

    // Test grep with very large thread count (should work, capped at file count)
    let mut args = vec!["grep".to_string(), "apple".to_string()];
    args.extend(files.iter().cloned());
    let output = idlebox_command()
        .args(&args)
        .args(["-j", "999"])
        .output()
        .expect("failed to execute process");
    assert!(output.status.success());

    // Test find with very large thread count
    let output = idlebox_command()
        .args(["find", tmp_dir.to_str().unwrap(), "-type", "f", "-j", "999"])
        .output()
        .expect("failed to execute process");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim().lines().count(), 3);

    // Test wc with very large thread count
    let mut args = vec!["wc".to_string(), "-l".to_string()];
    args.extend(files.iter().cloned());
    let output = idlebox_command()
        .args(&args)
        .args(["-j", "999"])
        .output()
        .expect("failed to execute process");
    assert!(output.status.success());

    let _ = fs::remove_dir_all(&tmp_dir);
}
