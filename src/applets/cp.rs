use crate::core::Applet;
use std::fs;
use std::io;
use std::path::Path;

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
            eprintln!("cp: missing destination operand");
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

            if src_path.is_dir() {
                if !recursive {
                    eprintln!("cp: -r not specified; omitting directory '{}'", src);
                    had_error = true;
                    continue;
                }
                if let Err(e) = Self::copy_dir_recursive(src_path, &target, force) {
                    eprintln!("cp: error copying '{}' to '{}': {}", src, dest, e);
                    had_error = true;
                }
            } else if src_path.is_file() || src_path.symlink_metadata().is_ok() {
                if let Err(e) = Self::copy_file(src_path, &target, force) {
                    eprintln!("cp: error copying '{}' to '{}': {}", src, dest, e);
                    had_error = true;
                }
            } else {
                eprintln!("cp: cannot stat '{}': No such file or directory", src);
                had_error = true;
            }
        }

        if had_error { Ok(1) } else { Ok(0) }
    }

    fn help(&self) {
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
    fn copy_file(src: &Path, dest: &Path, force: bool) -> io::Result<()> {
        if dest.exists() && !force {
            if let Some(parent) = dest.parent() {
                if parent.exists() {
                    // file exists, no force — still overwrite (POSIX default for cp without -i)
                }
            }
        }
        fs::copy(src, dest)?;
        Ok(())
    }

    fn copy_dir_recursive(src: &Path, dest: &Path, force: bool) -> io::Result<()> {
        fs::create_dir_all(dest)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());

            if src_path.is_dir() {
                Self::copy_dir_recursive(&src_path, &dest_path, force)?;
            } else {
                Self::copy_file(&src_path, &dest_path, force)?;
            }
        }

        Ok(())
    }
}
