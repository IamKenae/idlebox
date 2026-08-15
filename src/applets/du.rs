use crate::core::{human_size, Applet};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

pub struct DuApplet;

impl Applet for DuApplet {
    fn name(&self) -> &'static str {
        "du"
    }

    fn description(&self) -> &'static str {
        "Estimate file space usage"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut human_readable = false;
        let mut summarize = false;
        let mut max_depth: Option<usize> = None;
        let mut paths: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-h" | "--human-readable" => human_readable = true,
                "-s" | "--summarize" => summarize = true,
                _ if args[i].starts_with("-d") => {
                    let val = if args[i].len() > 2 {
                        args[i][2..].to_string()
                    } else {
                        i += 1;
                        if i < args.len() {
                            args[i].clone()
                        } else {
                            eprintln!("du: option '-d' requires an argument");
                            return Ok(1);
                        }
                    };
                    max_depth = Some(match val.parse::<usize>() {
                        Ok(n) => n,
                        Err(_) => {
                            eprintln!("du: invalid depth '{}'", val);
                            return Ok(1);
                        }
                    });
                }
                _ if args[i].starts_with("--max-depth=") => {
                    let val = &args[i]["--max-depth=".len()..];
                    max_depth = Some(match val.parse::<usize>() {
                        Ok(n) => n,
                        Err(_) => {
                            eprintln!("du: invalid depth '{}'", val);
                            return Ok(1);
                        }
                    });
                }
                _ if args[i].starts_with('-') && args[i].len() > 1 => {
                    let mut combined = true;
                    for ch in args[i][1..].chars() {
                        match ch {
                            'h' => human_readable = true,
                            's' => summarize = true,
                            _ => {
                                combined = false;
                                break;
                            }
                        }
                    }
                    if !combined {
                        eprintln!("du: invalid option -- '{}'", &args[i][1..]);
                        return Ok(1);
                    }
                }
                _ => {
                    paths.push(&args[i]);
                }
            }
            i += 1;
        }

        if paths.is_empty() {
            paths.push(".");
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();
        let mut exit_code = 0;

        for path in &paths {
            let p = Path::new(path);
            if p.symlink_metadata().is_err() {
                eprintln!("du: cannot access '{}': No such file or directory", path);
                exit_code = 1;
                continue;
            }

            if summarize {
                let total = dir_size(p)?;
                writeln!(out, "{}\t{}", format_size(total, human_readable), path)?;
            } else {
                match du_tree(p, 0, max_depth, human_readable, &mut out) {
                    Ok(total) => {
                        writeln!(out, "{}\t{}", format_size(total, human_readable), path)?;
                    }
                    Err(e) => {
                        eprintln!("du: error reading '{}': {}", path, e);
                        exit_code = 1;
                    }
                }
            }
        }

        Ok(exit_code)
    }

    fn help(&self) {
        println!("Usage: du [OPTION]... [FILE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -h, --human-readable  Print sizes in human readable format");
        println!("  -s, --summarize       Display only a total for each argument");
        println!("  -d, --max-depth N     Print total for a directory only if it is N or fewer levels below the argument");
    }
}

#[cfg(unix)]
fn disk_usage(metadata: &fs::Metadata) -> u64 {
    metadata.blocks() * 512
}

#[cfg(not(unix))]
fn disk_usage(metadata: &fs::Metadata) -> u64 {
    metadata.len()
}

fn dir_size(path: &Path) -> Result<u64, io::Error> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() {
        return Ok(disk_usage(&metadata));
    }

    let mut total = disk_usage(&metadata);

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let em = fs::symlink_metadata(entry.path())?;
        if em.is_dir() {
            total += dir_size(&entry.path())?;
        } else {
            total += disk_usage(&em);
        }
    }

    Ok(total)
}

fn du_tree(
    path: &Path,
    current_depth: usize,
    max_depth: Option<usize>,
    human_readable: bool,
    out: &mut impl Write,
) -> Result<u64, io::Error> {
    let metadata = fs::symlink_metadata(path)?;

    if !metadata.is_dir() {
        return Ok(disk_usage(&metadata));
    }

    let mut total = disk_usage(&metadata);

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let em = fs::symlink_metadata(entry.path())?;
        if em.is_dir() {
            let child_total = du_tree(
                &entry.path(),
                current_depth + 1,
                max_depth,
                human_readable,
                out,
            )?;
            total += child_total;
            let should_print = if let Some(md) = max_depth {
                current_depth < md
            } else {
                true
            };
            if should_print {
                writeln!(
                    out,
                    "{}\t{}",
                    format_size(child_total, human_readable),
                    entry.path().to_string_lossy()
                )?;
            }
        } else {
            total += disk_usage(&em);
        }
    }

    Ok(total)
}

fn format_size(bytes: u64, human_readable: bool) -> String {
    if !human_readable {
        let kb = bytes / 1024;
        return format!("{}", kb);
    }

    human_size(bytes, true, false)
}
