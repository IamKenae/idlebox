use crate::core::Applet;
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
            eprintln!("mv: missing destination operand");
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
                Err(e) if e.raw_os_error() == Some(18) => {
                    // EXDEV — cross-device link
                    if let Err(e2) = Self::move_cross_device(src_path, &target) {
                        eprintln!("mv: cannot move '{}' to '{}': {}", src, dest, e2);
                        had_error = true;
                    }
                }
                Err(e) => {
                    if src_path.is_dir() && e.raw_os_error().is_none() {
                        if let Err(e2) = Self::move_cross_device(src_path, &target) {
                            eprintln!("mv: cannot move '{}' to '{}': {}", src, dest, e2);
                            had_error = true;
                        }
                    } else {
                        eprintln!("mv: cannot rename '{}' to '{}': {}", src, dest, e);
                        had_error = true;
                    }
                }
            }
        }

        if had_error { Ok(1) } else { Ok(0) }
    }

    fn help(&self) {
        println!("Usage: mv [OPTION]... SOURCE... DEST");
        println!();
        println!("{}", self.description());
        println!();
        println!("If DEST is a directory, SOURCE(s) are moved into DEST.");
        println!("Handles cross-device moves automatically (copy + remove).");
    }
}

impl MvApplet {
    fn move_cross_device(src: &Path, dest: &Path) -> io::Result<()> {
        if src.is_dir() {
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

            if src_path.is_dir() {
                Self::copy_dir_recursive(&src_path, &dest_path)?;
            } else {
                fs::copy(&src_path, &dest_path)?;
            }
        }

        Ok(())
    }
}
