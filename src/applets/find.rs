use crate::core::Applet;
use std::fs;
use std::path::Path;

pub struct FindApplet;

impl Applet for FindApplet {
    fn name(&self) -> &'static str {
        "find"
    }

    fn description(&self) -> &'static str {
        "Search for files in a directory hierarchy"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut paths = Vec::new();
        let mut name_pattern: Option<String> = None;
        let mut type_filter: Option<char> = None;
        let mut max_depth: Option<usize> = None;
        let mut empty_only = false;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-name" => {
                    i += 1;
                    if i >= args.len() {
                        eprintln!("find: missing argument for -name");
                        return Ok(1);
                    }
                    name_pattern = Some(args[i].clone());
                }
                "-type" => {
                    i += 1;
                    if i >= args.len() {
                        eprintln!("find: missing argument for -type");
                        return Ok(1);
                    }
                    let mut chars = args[i].chars();
                    let t = chars.next().unwrap_or('\0');
                    if !matches!(t, 'f' | 'd' | 'l') || chars.next().is_some() {
                        eprintln!("find: unknown file type: {}", args[i]);
                        return Ok(1);
                    }
                    type_filter = Some(t);
                }
                "-maxdepth" => {
                    i += 1;
                    if i >= args.len() {
                        eprintln!("find: missing argument for -maxdepth");
                        return Ok(1);
                    }
                    max_depth = match args[i].parse::<usize>() {
                        Ok(depth) => Some(depth),
                        Err(_) => {
                            eprintln!("find: invalid -maxdepth value: {}", args[i]);
                            return Ok(1);
                        }
                    };
                }
                "-empty" => {
                    empty_only = true;
                }
                _ => {
                    if args[i].starts_with('-') {
                        eprintln!("find: unknown option: {}", args[i]);
                        return Ok(1);
                    }
                    paths.push(args[i].clone());
                }
            }
            i += 1;
        }

        if paths.is_empty() {
            paths.push(".".to_string());
        }

        for path in &paths {
            find_recursive(
                Path::new(path),
                &name_pattern,
                &type_filter,
                &max_depth,
                empty_only,
                0,
            )?;
        }

        Ok(0)
    }

    fn help(&self) {
        println!("Usage: find [PATH...] [OPTIONS]");
        println!();
        println!("Search for files in a directory hierarchy.");
        println!();
        println!("Options:");
        println!("  -name PATTERN  match file name with glob pattern");
        println!("  -type TYPE     filter by type: f (file), d (directory), l (symlink)");
        println!("  -maxdepth N    limit recursion depth");
        println!("  -empty         match only empty files or directories");
        println!();
        println!("Examples:");
        println!("  find . -name '*.rs'");
        println!("  find /tmp -type f -maxdepth 2");
        println!("  find . -empty -type d");
    }
}

fn find_recursive(
    path: &Path,
    name_pattern: &Option<String>,
    type_filter: &Option<char>,
    max_depth: &Option<usize>,
    empty_only: bool,
    current_depth: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(max) = max_depth {
        if current_depth > *max {
            return Ok(());
        }
    }

    let metadata = fs::symlink_metadata(path)?;
    let file_type = metadata.file_type();

    let is_match = check_match(path, &metadata, name_pattern, type_filter, empty_only)?;

    if is_match {
        println!("{}", path.display());
    }

    if file_type.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(path)?.collect::<Result<_, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            let entry_path = entry.path();
            find_recursive(
                &entry_path,
                name_pattern,
                type_filter,
                max_depth,
                empty_only,
                current_depth + 1,
            )?;
        }
    }

    Ok(())
}

fn check_match(
    path: &Path,
    metadata: &fs::Metadata,
    name_pattern: &Option<String>,
    type_filter: &Option<char>,
    empty_only: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    if let Some(pattern) = name_pattern {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !glob_match(pattern, file_name) {
            return Ok(false);
        }
    }

    if let Some(t) = type_filter {
        let file_type = metadata.file_type();
        let matches = match t {
            'f' => file_type.is_file(),
            'd' => file_type.is_dir(),
            'l' => file_type.is_symlink(),
            _ => false,
        };
        if !matches {
            return Ok(false);
        }
    }

    if empty_only {
        let file_type = metadata.file_type();
        if file_type.is_file() {
            if metadata.len() != 0 {
                return Ok(false);
            }
        } else if file_type.is_dir() {
            if fs::read_dir(path)?.next().transpose()?.is_some() {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
    }

    Ok(true)
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let p_chars = pattern.chars().peekable();
    let t_chars = text.chars().peekable();

    glob_match_inner(&p_chars.collect::<Vec<_>>(), &t_chars.collect::<Vec<_>>())
}

fn glob_match_inner(pattern: &[char], text: &[char]) -> bool {
    let mut pi = 0;
    let mut ti = 0;
    let mut star_pi = None;
    let mut star_ti = None;

    while ti < text.len() {
        if pi < pattern.len() && (pattern[pi] == '?' || pattern[pi] == text[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < pattern.len() && pattern[pi] == '*' {
            star_pi = Some(pi);
            star_ti = Some(ti);
            pi += 1;
        } else if let Some(spi) = star_pi {
            pi = spi + 1;
            star_ti = star_ti.map(|t| t + 1);
            ti = star_ti.unwrap();
        } else {
            return false;
        }
    }

    while pi < pattern.len() && pattern[pi] == '*' {
        pi += 1;
    }

    pi == pattern.len()
}
