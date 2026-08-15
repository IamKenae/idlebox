use crate::core::{
    banner,
    file_ops::{replace_file, same_file, unique_sibling_path, FollowSymlinks},
    Applet,
};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub struct LnApplet;

impl Applet for LnApplet {
    fn name(&self) -> &'static str {
        "ln"
    }

    fn description(&self) -> &'static str {
        "Create links between files"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut symbolic = false;
        let mut force = false;
        let mut positional: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-s" | "--symbolic" => symbolic = true,
                "-f" | "--force" => force = true,
                "-sf" | "-fs" => {
                    symbolic = true;
                    force = true;
                }
                "--" => {
                    positional.extend(args[i + 1..].iter().map(String::as_str));
                    break;
                }
                _ if args[i].starts_with('-') && args[i].len() > 1 => {
                    let mut combined = true;
                    for ch in args[i][1..].chars() {
                        match ch {
                            's' => symbolic = true,
                            'f' => force = true,
                            _ => {
                                combined = false;
                                break;
                            }
                        }
                    }
                    if !combined {
                        eprintln!("ln: invalid option -- '{}'", &args[i][1..]);
                        return Ok(1);
                    }
                }
                _ => positional.push(&args[i]),
            }
            i += 1;
        }

        if positional.len() < 2 {
            self.print_usage();
            return Ok(1);
        }

        let target = positional[positional.len() - 1];
        let sources = &positional[..positional.len() - 1];

        let target_is_dir = Path::new(target).is_dir();

        if sources.len() > 1 && !target_is_dir {
            eprintln!("ln: target '{}' is not a directory", target);
            return Ok(1);
        }

        let mut failed = false;

        for src in sources {
            let link_path = if target_is_dir {
                let src_name = Path::new(src)
                    .file_name()
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| (*src).into());
                Path::new(target).join(src_name)
            } else {
                PathBuf::from(target)
            };

            let existing = match fs::symlink_metadata(&link_path) {
                Ok(metadata) => Some(metadata),
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => {
                    eprintln!(
                        "ln: failed to inspect {} link '{}': {}",
                        if symbolic { "symbolic" } else { "hard" },
                        link_path.display(),
                        error
                    );
                    failed = true;
                    continue;
                }
            };

            if existing.is_some() && !force {
                eprintln!(
                    "ln: failed to create {} link '{}': File exists",
                    if symbolic { "symbolic" } else { "hard" },
                    link_path.display()
                );
                failed = true;
                continue;
            }

            if existing.is_some() {
                match same_file(Path::new(src), &link_path, FollowSymlinks::No) {
                    Ok(true) => {
                        eprintln!(
                            "ln: '{}' and '{}' are the same file",
                            src,
                            link_path.display()
                        );
                        failed = true;
                        continue;
                    }
                    Ok(false) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    // A symbolic-link target may intentionally be inaccessible;
                    // creating the link does not require reading that target.
                    Err(_) if symbolic => {}
                    Err(error) => {
                        eprintln!(
                            "ln: failed to compare '{}' and '{}': {}",
                            src,
                            link_path.display(),
                            error
                        );
                        failed = true;
                        continue;
                    }
                }
            }

            let result = if existing.is_some() {
                Self::replace_link(src, &link_path, symbolic)
            } else {
                Self::create_link(src, &link_path, symbolic)
            };

            if let Err(e) = result {
                eprintln!(
                    "ln: failed to create {} link '{}' -> '{}': {}",
                    if symbolic { "symbolic" } else { "hard" },
                    link_path.display(),
                    src,
                    e
                );
                failed = true;
            }
        }

        if failed {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    fn help(&self) {
        println!("{}", banner());
        println!();
        println!("Usage: ln [OPTION]... TARGET LINK_NAME");
        println!("   or: ln [OPTION]... TARGET... DIRECTORY");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -s, --symbolic   Create a symbolic link");
        println!("  -f, --force      Remove existing destination files");
    }
}

impl LnApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: ln [OPTION]... TARGET LINK_NAME");
        eprintln!("   or: ln [OPTION]... TARGET... DIRECTORY");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -s, --symbolic   Create a symbolic link");
        eprintln!("  -f, --force      Remove existing destination files");
    }

    fn create_link(src: &str, destination: &Path, symbolic: bool) -> io::Result<()> {
        if symbolic {
            create_symlink(src, destination)
        } else {
            fs::hard_link(src, destination)
        }
    }

    fn replace_link(src: &str, destination: &Path, symbolic: bool) -> io::Result<()> {
        let staged_path = unique_sibling_path(destination, "link")?;
        Self::create_link(src, &staged_path, symbolic)?;

        if !symbolic {
            match same_file(&staged_path, destination, FollowSymlinks::No) {
                Ok(true) => {
                    let _ = fs::remove_file(&staged_path);
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "'{}' and '{}' are the same file",
                            src,
                            destination.display()
                        ),
                    ));
                }
                Ok(false) => {}
                Err(error) => {
                    let _ = fs::remove_file(&staged_path);
                    return Err(error);
                }
            }
        }

        match replace_file(&staged_path, destination) {
            Ok(warning) => {
                if let Some(warning) = warning {
                    eprintln!(
                        "ln: warning: link was created, but old backup '{}' could not be removed: {}",
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
}

#[cfg(unix)]
fn create_symlink(src: &str, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn create_symlink(src: &str, dst: &Path) -> std::io::Result<()> {
    let src_path = Path::new(src);
    if src_path.is_dir() {
        std::os::windows::fs::symlink_dir(src, dst)
    } else {
        std::os::windows::fs::symlink_file(src, dst)
    }
}

#[cfg(not(any(unix, windows)))]
fn create_symlink(_src: &str, _dst: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "symlinks not supported",
    ))
}
