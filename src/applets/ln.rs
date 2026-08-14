use crate::core::Applet;
use std::fs;
use std::path::Path;

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
            eprintln!("ln: missing file operand");
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
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| src.to_string());
                Path::new(target).join(&src_name).to_string_lossy().to_string()
            } else {
                target.to_string()
            };

            let link = Path::new(&link_path);

            if link.exists() || link.symlink_metadata().is_ok() {
                if force {
                    let _ = fs::remove_file(link);
                } else {
                    eprintln!("ln: failed to create {} link '{}': File exists",
                        if symbolic { "symbolic" } else { "hard" }, link_path);
                    failed = true;
                    continue;
                }
            }

            let result = if symbolic {
                create_symlink(src, link)
            } else {
                fs::hard_link(src, link)
            };

            if let Err(e) = result {
                eprintln!("ln: failed to create {} link '{}' -> '{}': {}",
                    if symbolic { "symbolic" } else { "hard" }, link_path, src, e);
                failed = true;
            }
        }

        if failed { Ok(1) } else { Ok(0) }
    }

    fn help(&self) {
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
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "symlinks not supported"))
}
