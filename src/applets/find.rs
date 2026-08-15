use crate::core::Applet;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

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
        let mut num_threads: Option<usize> = None;

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
                "-j" | "--threads" => {
                    i += 1;
                    if i >= args.len() {
                        eprintln!("find: missing argument for -j");
                        return Ok(1);
                    }
                    num_threads = Some(match args[i].parse::<usize>() {
                        Ok(n) if n > 0 => n,
                        _ => {
                            eprintln!("find: invalid thread count: {}", args[i]);
                            return Ok(1);
                        }
                    });
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

        let num_threads = num_threads.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        });

        if num_threads <= 1 {
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
        } else {
            find_parallel(
                &paths,
                &name_pattern,
                &type_filter,
                &max_depth,
                empty_only,
                num_threads,
            )
        }
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
        println!("  -j, --threads N  use N threads for parallel search (default: auto)");
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
        entries.sort_unstable_by_key(|entry| entry.file_name());

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

fn find_parallel(
    paths: &[String],
    name_pattern: &Option<String>,
    type_filter: &Option<char>,
    max_depth: &Option<usize>,
    empty_only: bool,
    num_threads: usize,
) -> Result<i32, Box<dyn std::error::Error>> {
    let options = Arc::new(FindOptions {
        name_pattern: name_pattern.clone(),
        type_filter: *type_filter,
        max_depth: *max_depth,
        empty_only,
    });

    let work_queue = Arc::new(Mutex::new(VecDeque::<(PathBuf, usize)>::new()));
    {
        let mut queue = work_queue.lock().unwrap_or_else(|e| e.into_inner());
        for path in paths {
            queue.push_back((PathBuf::from(path), 0));
        }
    }

    let (tx, rx) = mpsc::channel::<(Vec<PathBuf>, bool)>();
    let active_threads = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..num_threads {
        let work_queue = Arc::clone(&work_queue);
        let options = Arc::clone(&options);
        let tx = tx.clone();
        let active_threads = Arc::clone(&active_threads);

        let handle = thread::spawn(move || {
            let mut local_results = Vec::new();
            let mut had_error = false;

            loop {
                let (path, depth) = {
                    let mut queue = work_queue.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(item) = queue.pop_front() {
                        active_threads.fetch_add(1, Ordering::AcqRel);
                        item
                    } else {
                        // Re-check under lock to avoid race with concurrent push
                        if active_threads.load(Ordering::Acquire) == 0 {
                            break;
                        }
                        drop(queue);
                        std::thread::yield_now();
                        continue;
                    }
                };

                if let Some(max) = options.max_depth {
                    if depth > max {
                        active_threads.fetch_sub(1, Ordering::AcqRel);
                        continue;
                    }
                }

                let metadata = match fs::symlink_metadata(&path) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("find: {}: {}", path.display(), e);
                        had_error = true;
                        active_threads.fetch_sub(1, Ordering::AcqRel);
                        continue;
                    }
                };

                let is_match = match check_match(
                    &path,
                    &metadata,
                    &options.name_pattern,
                    &options.type_filter,
                    options.empty_only,
                ) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("find: {}: {}", path.display(), e);
                        had_error = true;
                        active_threads.fetch_sub(1, Ordering::AcqRel);
                        continue;
                    }
                };

                if is_match {
                    local_results.push(path.clone());
                }

                if metadata.file_type().is_dir() {
                    let entries = match fs::read_dir(&path) {
                        Ok(e) => e,
                        Err(e) => {
                            eprintln!("find: {}: {}", path.display(), e);
                            had_error = true;
                            active_threads.fetch_sub(1, Ordering::AcqRel);
                            continue;
                        }
                    };

                    let mut subdirs: Vec<PathBuf> = Vec::new();
                    for entry in entries {
                        match entry {
                            Ok(e) => subdirs.push(e.path()),
                            Err(e) => {
                                eprintln!("find: {}", e);
                                had_error = true;
                            }
                        }
                    }
                    subdirs.sort_unstable_by(|a, b| a.file_name().cmp(&b.file_name()));

                    {
                        let mut queue = work_queue.lock().unwrap_or_else(|e| e.into_inner());
                        for subdir in subdirs {
                            queue.push_back((subdir, depth + 1));
                        }
                    }
                }

                active_threads.fetch_sub(1, Ordering::AcqRel);
            }

            tx.send((local_results, had_error)).ok();
        });
        handles.push(handle);
    }

    drop(tx);

    let mut all_results: Vec<PathBuf> = Vec::new();
    let mut had_error = false;
    for (results, thread_had_error) in rx {
        all_results.extend(results);
        if thread_had_error {
            had_error = true;
        }
    }

    for handle in handles {
        if let Err(e) = handle.join() {
            eprintln!("find: worker thread panicked: {:?}", e);
            had_error = true;
        }
    }

    all_results.sort();
    for path in all_results {
        println!("{}", path.display());
    }

    Ok(if had_error { 1 } else { 0 })
}

struct FindOptions {
    name_pattern: Option<String>,
    type_filter: Option<char>,
    max_depth: Option<usize>,
    empty_only: bool,
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
