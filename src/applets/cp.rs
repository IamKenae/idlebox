use crate::core::{
    banner,
    file_ops::{replace_file, same_file, unique_sibling_path, FollowSymlinks},
    Applet,
};
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Component, Path, PathBuf};

pub struct CpApplet;

impl Applet for CpApplet {
    fn name(&self) -> &'static str {
        "cp"
    }

    fn description(&self) -> &'static str {
        "Copy files and directories"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut recursive = false;
        let mut force = false;
        let mut sources: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "-r" | "-R" | "--recursive" => recursive = true,
                "-f" | "--force" => force = true,
                "-rf" | "-fr" | "-Rf" | "-fR" => {
                    recursive = true;
                    force = true;
                }
                "--" => {
                    sources.extend(args[i + 1..].iter().map(|s| s.as_str()));
                    break;
                }
                _ if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") => {
                    for ch in arg[1..].chars() {
                        match ch {
                            'r' | 'R' => recursive = true,
                            'f' => force = true,
                            _ => return Err(format!("cp: invalid option -- '{}'", ch).into()),
                        }
                    }
                }
                _ => sources.push(arg),
            }
            i += 1;
        }

        if sources.len() < 2 {
            self.print_usage();
            return Ok(1);
        }

        let dest = sources[sources.len() - 1];
        let srcs = &sources[..sources.len() - 1];
        let dest_path = Path::new(dest);
        let dest_is_dir = dest_path.is_dir() || srcs.len() > 1;

        if srcs.len() > 1 && dest_path.exists() && !dest_path.is_dir() {
            eprintln!("cp: target '{}' is not a directory", dest);
            return Ok(1);
        }

        let mut had_error = false;

        for src in srcs {
            let src_path = Path::new(src);
            let target = if dest_is_dir {
                let file_name = src_path.file_name().unwrap_or(src_path.as_os_str());
                dest_path.join(file_name)
            } else {
                dest_path.to_path_buf()
            };

            let metadata = match fs::symlink_metadata(src_path) {
                Ok(metadata) => metadata,
                Err(_) => {
                    eprintln!("cp: cannot stat '{}': No such file or directory", src);
                    had_error = true;
                    continue;
                }
            };

            if metadata.is_dir() {
                if !recursive {
                    eprintln!("cp: -r not specified; omitting directory '{}'", src);
                    had_error = true;
                    continue;
                }
                if let Err(e) = Self::ensure_destination_outside_source(src_path, &target) {
                    eprintln!("cp: error copying '{}' to '{}': {}", src, dest, e);
                    had_error = true;
                    continue;
                }
                if let Err(e) = Self::copy_dir_recursive(src_path, &target, force) {
                    eprintln!("cp: error copying '{}' to '{}': {}", src, dest, e);
                    had_error = true;
                }
            } else if metadata.file_type().is_symlink() {
                if let Err(e) = Self::copy_symlink(src_path, &target) {
                    eprintln!("cp: error copying '{}' to '{}': {}", src, dest, e);
                    had_error = true;
                }
            } else if metadata.is_file() {
                if let Err(e) = Self::copy_file(src_path, &target, force) {
                    eprintln!("cp: error copying '{}' to '{}': {}", src, dest, e);
                    had_error = true;
                }
            } else {
                eprintln!("cp: cannot stat '{}': No such file or directory", src);
                had_error = true;
            }
        }

        if had_error {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    fn help(&self) {
        println!("{}", banner());
        println!();
        println!("Usage: cp [OPTION]... SOURCE... DEST");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -r, -R, --recursive    Copy directories recursively");
        println!("  -f, --force            Force overwrite of existing destination files");
    }
}

impl CpApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: cp [OPTION]... SOURCE... DEST");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -r, -R, --recursive    Copy directories recursively");
        eprintln!("  -f, --force            Force overwrite of existing destination files");
    }

    fn copy_file(src: &Path, dest: &Path, force: bool) -> io::Result<()> {
        if same_file(src, dest, FollowSymlinks::Yes)? {
            return Err(Self::same_file_error(src, dest));
        }

        let destination = match fs::symlink_metadata(dest) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };

        if force && destination.is_some_and(|metadata| !metadata.is_dir()) {
            return Self::copy_file_staged(src, dest);
        }

        fs::copy(src, dest)?;
        Ok(())
    }

    fn copy_file_staged(src: &Path, dest: &Path) -> io::Result<()> {
        let staged_path = unique_sibling_path(dest, "copy")?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged_path)?;

        if let Err(error) = fs::copy(src, &staged_path) {
            let _ = fs::remove_file(&staged_path);
            return Err(error);
        }

        match replace_file(&staged_path, dest) {
            Ok(warning) => {
                if let Some(warning) = warning {
                    eprintln!(
                        "cp: warning: copied '{}', but old backup '{}' could not be removed: {}",
                        dest.display(),
                        warning.backup_path.display(),
                        warning.error
                    );
                }
                Ok(())
            }
            Err(error) => {
                let _ = fs::remove_file(&staged_path);
                Err(error)
            }
        }
    }

    fn copy_dir_recursive(src: &Path, dest: &Path, force: bool) -> io::Result<()> {
        fs::create_dir_all(dest)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());

            let metadata = fs::symlink_metadata(&src_path)?;
            if metadata.is_dir() {
                Self::copy_dir_recursive(&src_path, &dest_path, force)?;
            } else if metadata.file_type().is_symlink() {
                Self::copy_symlink(&src_path, &dest_path)?;
            } else {
                Self::copy_file(&src_path, &dest_path, force)?;
            }
        }

        Ok(())
    }

    fn copy_symlink(src: &Path, dest: &Path) -> io::Result<()> {
        if same_file(src, dest, FollowSymlinks::No)? {
            return Err(Self::same_file_error(src, dest));
        }

        let target = fs::read_link(src)?;
        match fs::symlink_metadata(dest) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "destination is an existing directory",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return create_symlink(&target, dest, src);
            }
            Err(error) => return Err(error),
        }

        let staged_path = unique_sibling_path(dest, "copy")?;
        create_symlink(&target, &staged_path, src)?;
        match replace_file(&staged_path, dest) {
            Ok(warning) => {
                if let Some(warning) = warning {
                    eprintln!(
                        "cp: warning: copied '{}', but old backup '{}' could not be removed: {}",
                        dest.display(),
                        warning.backup_path.display(),
                        warning.error
                    );
                }
                Ok(())
            }
            Err(error) => {
                let _ = fs::remove_file(&staged_path);
                Err(error)
            }
        }
    }

    fn same_file_error(src: &Path, dest: &Path) -> io::Error {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "'{}' and '{}' are the same file",
                src.display(),
                dest.display()
            ),
        )
    }

    fn ensure_destination_outside_source(src: &Path, dest: &Path) -> io::Result<()> {
        let source = fs::canonicalize(src)?;
        let destination = Self::resolve_for_comparison(dest)?;
        if destination == source || destination.starts_with(&source) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot copy a directory into itself",
            ));
        }
        Ok(())
    }

    fn resolve_for_comparison(path: &Path) -> io::Result<PathBuf> {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };
        let normalized = normalize_path(&absolute);

        let mut ancestor = normalized.as_path();
        let mut suffix = Vec::new();
        loop {
            if let Ok(mut resolved) = fs::canonicalize(ancestor) {
                for component in suffix.iter().rev() {
                    resolved.push(component);
                }
                return Ok(resolved);
            }

            let name = ancestor.file_name().ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "no existing destination ancestor")
            })?;
            suffix.push(name.to_os_string());
            ancestor = ancestor.parent().ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "no existing destination ancestor")
            })?;
        }
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

#[cfg(unix)]
fn create_symlink(target: &Path, dest: &Path, _source_link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, dest)
}

#[cfg(windows)]
fn create_symlink(target: &Path, dest: &Path, source_link: &Path) -> io::Result<()> {
    if fs::metadata(source_link).is_ok_and(|metadata| metadata.is_dir()) {
        std::os::windows::fs::symlink_dir(target, dest)
    } else {
        std::os::windows::fs::symlink_file(target, dest)
    }
}

#[cfg(not(any(unix, windows)))]
fn create_symlink(_target: &Path, _dest: &Path, _source_link: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "copying symbolic links is not supported on this platform",
    ))
}
