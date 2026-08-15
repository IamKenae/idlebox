use crate::core::{banner, Applet};
use std::fs;
use std::io;
use std::path::Path;

pub struct MvApplet;

impl Applet for MvApplet {
    fn name(&self) -> &'static str {
        "mv"
    }

    fn description(&self) -> &'static str {
        "Move (rename) files and directories"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut sources: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "--" => {
                    sources.extend(args[i + 1..].iter().map(|s| s.as_str()));
                    break;
                }
                _ if arg.starts_with('-') && arg.len() > 1 => {
                    return Err(format!("mv: invalid option -- '{}'", &arg[1..]).into());
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
            eprintln!("mv: target '{}' is not a directory", dest);
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

            if !src_path.exists() && src_path.symlink_metadata().is_err() {
                eprintln!("mv: cannot stat '{}': No such file or directory", src);
                had_error = true;
                continue;
            }

            match fs::rename(src_path, &target) {
                Ok(()) => {}
                Err(e) if is_cross_device_error(&e) => {
                    if let Err(e2) = Self::move_cross_device(src_path, &target) {
                        eprintln!("mv: cannot move '{}' to '{}': {}", src, dest, e2);
                        had_error = true;
                    }
                }
                Err(e) => {
                    eprintln!("mv: cannot rename '{}' to '{}': {}", src, dest, e);
                    had_error = true;
                }
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
        println!("Usage: mv [OPTION]... SOURCE... DEST");
        println!();
        println!("{}", self.description());
        println!();
        println!("If DEST is a directory, SOURCE(s) are moved into DEST.");
        println!("Handles cross-device moves automatically (copy + remove).");
    }
}

impl MvApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: mv [OPTION]... SOURCE... DEST");
        eprintln!();
        eprintln!("If DEST is a directory, SOURCE(s) are moved into DEST.");
        eprintln!("Handles cross-device moves automatically (copy + remove).");
    }

    fn move_cross_device(src: &Path, dest: &Path) -> io::Result<()> {
        let metadata = fs::symlink_metadata(src)?;
        if metadata.file_type().is_symlink() {
            Self::copy_symlink(src, dest)?;
            fs::remove_file(src)?;
        } else if metadata.is_dir() {
            Self::copy_dir_recursive(src, dest)?;
            fs::remove_dir_all(src)?;
        } else {
            fs::copy(src, dest)?;
            fs::remove_file(src)?;
        }
        Ok(())
    }

    fn copy_dir_recursive(src: &Path, dest: &Path) -> io::Result<()> {
        fs::create_dir_all(dest)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());

            let metadata = fs::symlink_metadata(&src_path)?;
            if metadata.is_dir() {
                Self::copy_dir_recursive(&src_path, &dest_path)?;
            } else if metadata.file_type().is_symlink() {
                Self::copy_symlink(&src_path, &dest_path)?;
            } else {
                fs::copy(&src_path, &dest_path)?;
            }
        }

        Ok(())
    }

    fn copy_symlink(src: &Path, dest: &Path) -> io::Result<()> {
        if let Ok(metadata) = fs::symlink_metadata(dest) {
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "destination is an existing directory",
                ));
            }
            fs::remove_file(dest)?;
        }
        let target = fs::read_link(src)?;
        create_symlink(&target, dest, src)
    }
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
        "moving symbolic links across devices is not supported on this platform",
    ))
}

#[cfg(unix)]
fn is_cross_device_error(e: &io::Error) -> bool {
    e.raw_os_error() == Some(libc::EXDEV)
}

#[cfg(unix)]
mod libc {
    pub const EXDEV: i32 = 18;
}

#[cfg(windows)]
fn is_cross_device_error(e: &io::Error) -> bool {
    e.raw_os_error() == Some(17)
}

#[cfg(not(any(unix, windows)))]
fn is_cross_device_error(_e: &io::Error) -> bool {
    false
}
