//! Tree-style directory listing.
//!
//! Mirrors the classic `tree(1)` layout: every directory level adds a prefix of
//! connector glyphs so the hierarchy stays readable, and the traversal caches
//! each entry's metadata so a file is only `stat`ed once no matter how many of
//! the `-s`/`-p`/`-u`/`-g`/`-D` columns are enabled.
//!
//! Names travel from `read_dir` to the output as raw `OsStr` bytes. A Unix file
//! name is a byte string that need not be UTF-8, so converting it early would
//! replace those bytes with `U+FFFD` and make two different files print — and
//! link — identically.

use crate::core::file_ops::{replace_file, same_file, unique_sibling_path, FollowSymlinks};
#[cfg(unix)]
use crate::core::unix_ffi::{lock_account_db, raw_getgrgid, raw_getpwuid};
use crate::core::{human_size, Applet};
use std::borrow::Cow;
use std::cmp::Ordering;
#[cfg(unix)]
use std::ffi::{c_char, CStr};
use std::ffi::{OsStr, OsString};
use std::fs::{self, Metadata, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};

pub struct TreeApplet;

/// Which serialization the tree is rendered as.
#[derive(Default)]
enum Format {
    #[default]
    Text,
    Json,
    Xml,
    Html,
}

/// Everything the command line can turn on. Kept as one struct so the recursive
/// walkers stay within a handful of parameters.
#[derive(Default)]
struct Options {
    all: bool,
    dirs_only: bool,
    max_depth: Option<usize>,
    // `-P`/`-I` accumulate: repeating either adds to the set, as upstream's
    // `patterns[pattern++] = argv[n++]` does.
    include: Vec<String>,
    exclude: Vec<String>,

    full_path: bool,
    no_indent: bool,
    classify: bool,
    color: bool,
    ascii: bool,

    size: bool,
    human: bool,
    perms: bool,
    user: bool,
    group: bool,
    mtime: bool,

    dirs_first: bool,
    reverse: bool,
    time_sort: bool,

    format: Format,
    html_base: String,
    no_report: bool,
    output: Option<String>,
    /// Where `-o` is being staged. The file sits next to the destination, so
    /// the walk has to skip it or the report would list its own scratch copy.
    staged_output: Option<PathBuf>,

    // Connector glyphs, resolved once from `--charset` and `-i`.
    branch: &'static str,
    last: &'static str,
    vertical: &'static str,
    blank: &'static str,
}

impl Options {
    /// Resolve the connector glyphs. `-i` wins over `--charset` because it drops
    /// the connector lines *and* the indentation at every depth, which is what
    /// makes `tree -if` a pipe-able list of bare paths.
    fn resolve_glyphs(&mut self) {
        let (branch, last, vertical, blank) = if self.no_indent {
            ("", "", "", "")
        } else if self.ascii {
            ("|-- ", "`-- ", "|   ", "    ")
        } else {
            ("├── ", "└── ", "│   ", "    ")
        };
        self.branch = branch;
        self.last = last;
        self.vertical = vertical;
        self.blank = blank;
    }

    fn any_meta(&self) -> bool {
        self.size || self.human || self.perms || self.user || self.group || self.mtime
    }
}

/// Running counters plus the "something went wrong" flag that decides the exit code.
#[derive(Default)]
struct State {
    dirs: usize,
    files: usize,
    failed: bool,
}

/// A directory entry with its metadata already read, so it is only `stat`ed once.
struct Entry {
    name: OsString,
    path: PathBuf,
    /// Always the `lstat` result: the columns and the `-J`/`-X` type describe
    /// the link itself, not what it points at.
    meta: Metadata,
    /// Directory-ness *through* a link, the way upstream's `getinfo()` decides
    /// it. Drives the counters, `-d`, `--dirsfirst` and whether to descend.
    is_dir: bool,
    /// Cached `readlink`, so all four renderers share the one syscall.
    link_target: Option<PathBuf>,
}

impl Entry {
    fn is_symlink(&self) -> bool {
        self.meta.file_type().is_symlink()
    }
}

impl Applet for TreeApplet {
    fn name(&self) -> &'static str {
        "tree"
    }

    fn description(&self) -> &'static str {
        "List directory contents in a tree-like format"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut opts = Options::default();
        let mut paths: Vec<String> = Vec::new();
        let mut no_more_opts = false;

        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];

            if no_more_opts || !arg.starts_with('-') || arg == "-" {
                paths.push(arg.clone());
                i += 1;
                continue;
            }

            if arg == "--" {
                no_more_opts = true;
                i += 1;
                continue;
            }

            if arg.starts_with("--") {
                // A value-taking long option accepts both `--charset X` and
                // `--charset=X`, the way upstream's `long_arg()` does.
                if let Some(value) = arg.strip_prefix("--charset=") {
                    if value.is_empty() {
                        eprintln!("tree: option '--charset' requires an argument");
                        return Ok(1);
                    }
                    if !Self::set_charset(value, &mut opts) {
                        return Ok(1);
                    }
                    i += 1;
                    continue;
                }

                match arg.as_str() {
                    "--help" => {
                        self.help();
                        return Ok(0);
                    }
                    "--dirsfirst" => opts.dirs_first = true,
                    "--noreport" => opts.no_report = true,
                    "--charset" => {
                        i += 1;
                        if i >= args.len() {
                            eprintln!("tree: option '--charset' requires an argument");
                            return Ok(1);
                        }
                        if !Self::set_charset(&args[i], &mut opts) {
                            return Ok(1);
                        }
                    }
                    _ => {
                        eprintln!("tree: unrecognized option '{}'", arg);
                        return Ok(1);
                    }
                }
                i += 1;
                continue;
            }

            // Short options, possibly bundled. An option that takes a value
            // consumes the rest of the bundle (`-L2`) or, when it ends the
            // bundle, the next argument (`-aL 2`) — the way getopt does it.
            let chars: Vec<char> = arg[1..].chars().collect();
            let mut idx = 0;
            while idx < chars.len() {
                let ch = chars[idx];
                match ch {
                    'a' => opts.all = true,
                    'd' => opts.dirs_only = true,
                    'f' => opts.full_path = true,
                    'i' => opts.no_indent = true,
                    's' => opts.size = true,
                    'h' => {
                        opts.human = true;
                        opts.size = true;
                    }
                    'p' => opts.perms = true,
                    'u' => opts.user = true,
                    'g' => opts.group = true,
                    'D' => opts.mtime = true,
                    'F' => opts.classify = true,
                    'C' => opts.color = true,
                    // Accepted and deliberately inert: colour is off unless -C
                    // asks for it, and -C outranks -n whatever the order —
                    // upstream's own help reads "-n  Turn colorization off
                    // always (-C overrides)". Nothing probes for a tty here, so
                    // there is no default colour for -n to switch off.
                    'n' => {}
                    'r' => opts.reverse = true,
                    't' => opts.time_sort = true,
                    'J' => opts.format = Format::Json,
                    'X' => opts.format = Format::Xml,
                    'L' | 'I' | 'P' | 'o' | 'H' => {
                        let value: String = if idx + 1 < chars.len() {
                            chars[idx + 1..].iter().collect()
                        } else {
                            i += 1;
                            if i >= args.len() {
                                eprintln!("tree: option requires an argument -- '{}'", ch);
                                return Ok(1);
                            }
                            args[i].clone()
                        };
                        match ch {
                            'L' => match value.parse::<usize>() {
                                Ok(0) | Err(_) => {
                                    eprintln!("tree: invalid level, must be greater than 0");
                                    return Ok(1);
                                }
                                Ok(n) => opts.max_depth = Some(n),
                            },
                            'I' => opts.exclude.push(value),
                            'P' => opts.include.push(value),
                            'o' => opts.output = Some(value),
                            _ => {
                                opts.format = Format::Html;
                                opts.html_base = value;
                            }
                        }
                        break;
                    }
                    _ => {
                        eprintln!("tree: invalid option -- '{}'", ch);
                        return Ok(1);
                    }
                }
                idx += 1;
            }
            i += 1;
        }

        opts.resolve_glyphs();

        if paths.is_empty() {
            paths.push(".".to_string());
        }

        match opts.output.clone() {
            Some(file) => Self::render_to_file(&file, &paths, &mut opts),
            None => {
                let mut out = io::BufWriter::new(io::stdout());
                let code = Self::render(&paths, &opts, &mut out)?;
                out.flush()?;
                Ok(code)
            }
        }
    }

    fn help(&self) {
        println!("Usage: tree [OPTION]... [DIRECTORY]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Listing options:");
        println!("  -a              Print all files, including hidden ones");
        println!("  -d              List directories only");
        println!("  -L LEVEL        Descend only LEVEL directories deep (LEVEL >= 1)");
        println!("  -f              Print the full path prefix for each entry (text output only)");
        println!("  -I PATTERN      Do not list entries matching PATTERN (repeatable)");
        println!("  -P PATTERN      List only files matching PATTERN (repeatable)");
        println!("  -i              Print entries without indentation or connector lines");
        println!();
        println!("File information:");
        println!("  -s              Print the size of each file in bytes");
        println!("  -h              Print the size of each file in human readable form");
        println!("  -p              Print the permissions of each file");
        println!("  -u              Print the owner name of each file");
        println!("  -g              Print the group name of each file");
        println!("  -D              Print the last modification time of each file (UTC)");
        println!("  -F              Append a type indicator (/, *, @, |, =) to each entry");
        println!();
        println!("Sorting:");
        println!("  --dirsfirst     List directories before files");
        println!("  -r              Sort in reverse order");
        println!("  -t              Sort by last modification time");
        println!();
        println!("Output:");
        println!("  -C              Colorize the output");
        println!("  -n              Do not colorize the output (default; -C overrides)");
        println!("  -o FILE         Write the output to FILE instead of standard output");
        println!("  -J              Print the tree as JSON");
        println!("  -X              Print the tree as XML");
        println!("  -H BASE         Print the tree as HTML, using BASE as the link prefix");
        println!("  --charset SET   Line-drawing character set: UTF-8 (default) or ASCII");
        println!("  --noreport      Omit the file and directory count at the end");
        println!();
        println!("An option that takes a value accepts it attached (-L2) or separate (-L 2);");
        println!("a long option also accepts --charset=ASCII.");
        println!();
        println!("A pattern matches one name and understands *, ?, [set], [^set], a-z ranges,");
        println!("\\ escapes and | alternation.");
    }
}

impl TreeApplet {
    /// `--charset X` and `--charset=X` share this. Returns whether the value
    /// was understood; the error has already been reported if not.
    fn set_charset(value: &str, opts: &mut Options) -> bool {
        match value.to_ascii_uppercase().as_str() {
            "ASCII" => {
                opts.ascii = true;
                true
            }
            "UTF-8" | "UTF8" => {
                opts.ascii = false;
                true
            }
            _ => {
                eprintln!("tree: unsupported charset: {}", value);
                false
            }
        }
    }

    /// `-o FILE`, written through a staging file next to the destination.
    ///
    /// Creating the destination up front would truncate it before the walk even
    /// starts, so `tree -o notes.txt notes.txt` would report on a file it had
    /// already emptied, and a walk that fails halfway would still have destroyed
    /// whatever was there. Staging and renaming mirrors what `cp`, `ln` and
    /// `--install` already do in this repo.
    fn render_to_file(
        file: &str,
        paths: &[String],
        opts: &mut Options,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let destination = PathBuf::from(file);

        // A directory is never a destination. Rejecting it up front keeps the
        // message the same whether or not it is also one of the inputs.
        if fs::symlink_metadata(&destination)
            .map(|meta| meta.file_type().is_dir())
            .unwrap_or(false)
        {
            eprintln!("tree: cannot open output file '{}': Is a directory", file);
            return Ok(1);
        }

        // Publishing over an input would swap the report in for the original,
        // including through a symlink or a hard link to it.
        for path in paths {
            match same_file(Path::new(path), &destination, FollowSymlinks::Yes) {
                Ok(true) => {
                    eprintln!("tree: '{}' is both an input and the output file", file);
                    return Ok(1);
                }
                // A missing input is reported by the walk, not here.
                Ok(false) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    eprintln!("tree: cannot open output file '{}': {}", file, error);
                    return Ok(1);
                }
            }
        }

        let staged = match unique_sibling_path(&destination, "tree") {
            Ok(path) => path,
            Err(error) => {
                eprintln!("tree: cannot open output file '{}': {}", file, error);
                return Ok(1);
            }
        };
        let handle = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)
        {
            Ok(handle) => handle,
            Err(error) => {
                eprintln!("tree: cannot open output file '{}': {}", file, error);
                return Ok(1);
            }
        };
        opts.staged_output = Some(staged.clone());

        // Scoped so the handle is closed before the rename: Windows will not
        // rename over a file that is still open.
        let rendered = {
            let mut out = io::BufWriter::new(handle);
            Self::render(paths, opts, &mut out).and_then(|code| out.flush().map(|()| code))
        };
        let code = match rendered {
            Ok(code) => code,
            Err(error) => {
                let _ = fs::remove_file(&staged);
                eprintln!("tree: cannot open output file '{}': {}", file, error);
                return Ok(1);
            }
        };

        match replace_file(&staged, &destination) {
            Ok(None) => Ok(code),
            Ok(Some(warning)) => {
                eprintln!(
                    "tree: warning: wrote '{}', but old backup '{}' could not be removed: {}",
                    file,
                    warning.backup_path.display(),
                    warning.error
                );
                Ok(code)
            }
            Err(error) => {
                let _ = fs::remove_file(&staged);
                eprintln!("tree: cannot open output file '{}': {}", file, error);
                Ok(1)
            }
        }
    }

    fn render(paths: &[String], opts: &Options, out: &mut dyn Write) -> io::Result<i32> {
        let mut st = State::default();

        match opts.format {
            Format::Text => {
                for (idx, path) in paths.iter().enumerate() {
                    if idx > 0 {
                        writeln!(out)?;
                    }
                    Self::text_root(path, opts, &mut st, out)?;
                }
                if !opts.no_report {
                    writeln!(out)?;
                    writeln!(out, "{}", Self::report_line(&st, opts))?;
                }
            }
            Format::Json => {
                writeln!(out, "[")?;
                // A root that fails to stat writes nothing, so the separator has
                // to be driven by what was actually emitted rather than by the
                // index — otherwise a failing last path leaves a trailing comma.
                let mut wrote_any = false;
                for path in paths {
                    if Self::json_root(path, wrote_any, opts, &mut st, out)? {
                        wrote_any = true;
                    }
                }
                if wrote_any {
                    // Terminate the last object, with a comma if the report follows.
                    if opts.no_report {
                        writeln!(out)?;
                    } else {
                        writeln!(out, ",")?;
                    }
                }
                if !opts.no_report {
                    write!(out, "  {{\"type\":\"report\",\"directories\":{}", st.dirs)?;
                    if !opts.dirs_only {
                        write!(out, ",\"files\":{}", st.files)?;
                    }
                    writeln!(out, "}}")?;
                }
                writeln!(out, "]")?;
            }
            Format::Xml => {
                writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
                writeln!(out, "<tree>")?;
                for path in paths {
                    Self::xml_root(path, opts, &mut st, out)?;
                }
                if !opts.no_report {
                    writeln!(out, "  <report>")?;
                    writeln!(out, "    <directories>{}</directories>", st.dirs)?;
                    if !opts.dirs_only {
                        writeln!(out, "    <files>{}</files>", st.files)?;
                    }
                    writeln!(out, "  </report>")?;
                }
                writeln!(out, "</tree>")?;
            }
            Format::Html => {
                Self::html_header(out)?;
                for path in paths {
                    Self::html_root(path, opts, &mut st, out)?;
                }
                if !opts.no_report {
                    writeln!(out, "<hr>")?;
                    writeln!(out, "<p>{}</p>", Self::report_line(&st, opts))?;
                }
                writeln!(out, "</body>")?;
                writeln!(out, "</html>")?;
            }
        }

        Ok(if st.failed { 1 } else { 0 })
    }

    fn report_line(st: &State, opts: &Options) -> String {
        let dirs = format!(
            "{} {}",
            st.dirs,
            if st.dirs == 1 {
                "directory"
            } else {
                "directories"
            }
        );
        if opts.dirs_only {
            return dirs;
        }
        format!(
            "{}, {} {}",
            dirs,
            st.files,
            if st.files == 1 { "file" } else { "files" }
        )
    }

    // -- text ------------------------------------------------------------

    fn text_root(
        path: &str,
        opts: &Options,
        st: &mut State,
        out: &mut dyn Write,
    ) -> io::Result<()> {
        let root = Path::new(path);
        let (meta, link) = match Self::root_meta(root) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("tree: {}: {}", path, e);
                st.failed = true;
                return Ok(());
            }
        };
        Self::count_root(&meta, st);

        Self::write_meta(&meta, opts, out)?;
        Self::write_colored(path.as_bytes(), &meta, root, opts, out)?;
        if opts.classify {
            Self::write_marker(&meta, root, out)?;
        }
        if let Some(target) = &link {
            out.write_all(b" -> ")?;
            out.write_all(&os_bytes(target.as_os_str()))?;
        }

        if !meta.is_dir() {
            writeln!(out)?;
            return Ok(());
        }

        match Self::read_children(root, opts, st) {
            Ok(children) => {
                writeln!(out)?;
                Self::walk_text(&children, "", 1, opts, st, out)?;
            }
            Err(_) => {
                writeln!(out, " [error opening dir]")?;
                st.failed = true;
            }
        }
        Ok(())
    }

    fn walk_text(
        children: &[Entry],
        prefix: &str,
        depth: usize,
        opts: &Options,
        st: &mut State,
        out: &mut dyn Write,
    ) -> io::Result<()> {
        for (idx, entry) in children.iter().enumerate() {
            let is_last = idx + 1 == children.len();
            Self::count(entry, st);

            let sub = Self::descend(entry, depth, opts, st);

            write!(out, "{}", prefix)?;
            write!(out, "{}", if is_last { opts.last } else { opts.branch })?;
            Self::write_meta(&entry.meta, opts, out)?;
            Self::write_entry_name(entry, opts, out)?;

            match sub {
                Some(Err(_)) => {
                    writeln!(out, " [error opening dir]")?;
                    st.failed = true;
                }
                Some(Ok(grandchildren)) => {
                    writeln!(out)?;
                    let next = format!(
                        "{}{}",
                        prefix,
                        if is_last { opts.blank } else { opts.vertical }
                    );
                    Self::walk_text(&grandchildren, &next, depth + 1, opts, st, out)?;
                }
                None => writeln!(out)?,
            }
        }
        Ok(())
    }

    // -- json ------------------------------------------------------------

    /// Writes one root object, without the newline that terminates it — the
    /// caller adds that once it knows whether a comma is needed. `separator`
    /// closes off the previous object. Returns whether anything was written.
    fn json_root(
        path: &str,
        separator: bool,
        opts: &Options,
        st: &mut State,
        out: &mut dyn Write,
    ) -> io::Result<bool> {
        let root = Path::new(path);
        let (meta, link) = match Self::root_meta(root) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("tree: {}: {}", path, e);
                st.failed = true;
                return Ok(false);
            }
        };
        Self::count_root(&meta, st);

        if separator {
            writeln!(out, ",")?;
        }

        write!(
            out,
            "  {{\"type\":\"{}\",\"name\":\"",
            root_type_name(&meta, &link)
        )?;
        write_json_escaped(out, path.as_bytes())?;
        write!(out, "\"")?;
        Self::json_link(link.as_deref(), out)?;
        Self::json_meta(&meta, opts, out)?;

        if !meta.is_dir() {
            write!(out, "}}")?;
            return Ok(true);
        }

        match Self::read_children(root, opts, st) {
            Ok(children) => {
                writeln!(out, ",\"contents\":[")?;
                Self::walk_json(&children, 4, 1, opts, st, out)?;
                write!(out, "  ]}}")?;
            }
            Err(_) => {
                write!(out, ",\"error\":\"opening dir\"}}")?;
                st.failed = true;
            }
        }
        Ok(true)
    }

    fn walk_json(
        children: &[Entry],
        indent: usize,
        depth: usize,
        opts: &Options,
        st: &mut State,
        out: &mut dyn Write,
    ) -> io::Result<()> {
        for (idx, entry) in children.iter().enumerate() {
            let trailing = if idx + 1 == children.len() { "" } else { "," };
            Self::count(entry, st);

            write!(out, "{:1$}", "", indent)?;
            write!(out, "{{\"type\":\"{}\",\"name\":\"", type_name(&entry.meta))?;
            write_json_escaped(out, &os_bytes(&entry.name))?;
            write!(out, "\"")?;
            Self::json_link(entry.link_target.as_deref(), out)?;
            Self::json_meta(&entry.meta, opts, out)?;

            match Self::descend(entry, depth, opts, st) {
                Some(Err(_)) => {
                    writeln!(out, ",\"error\":\"opening dir\"}}{}", trailing)?;
                    st.failed = true;
                }
                Some(Ok(grandchildren)) => {
                    writeln!(out, ",\"contents\":[")?;
                    Self::walk_json(&grandchildren, indent + 2, depth + 1, opts, st, out)?;
                    write!(out, "{:1$}", "", indent)?;
                    writeln!(out, "]}}{}", trailing)?;
                }
                None => writeln!(out, "}}{}", trailing)?,
            }
        }
        Ok(())
    }

    fn json_link(target: Option<&Path>, out: &mut dyn Write) -> io::Result<()> {
        let Some(target) = target else {
            return Ok(());
        };
        write!(out, ",\"target\":\"")?;
        write_json_escaped(out, &os_bytes(target.as_os_str()))?;
        write!(out, "\"")
    }

    fn json_meta(meta: &Metadata, opts: &Options, out: &mut dyn Write) -> io::Result<()> {
        if opts.perms {
            write!(
                out,
                ",\"mode\":\"{}\",\"prot\":\"{}\"",
                mode_field(meta),
                permission_field(meta)
            )?;
        }
        if opts.user {
            write!(out, ",\"user\":\"")?;
            write_json_escaped(out, owner_name(meta).as_bytes())?;
            write!(out, "\"")?;
        }
        if opts.group {
            write!(out, ",\"group\":\"")?;
            write_json_escaped(out, group_name(meta).as_bytes())?;
            write!(out, "\"")?;
        }
        if opts.size {
            // `-h` turns the number into a string here, the way upstream's
            // `json_fillinfo()` does; XML keeps raw bytes either way.
            if opts.human {
                write!(out, ",\"size\":\"{}\"", human_size(meta.len(), false, true))?;
            } else {
                write!(out, ",\"size\":{}", meta.len())?;
            }
        }
        if opts.mtime {
            write!(out, ",\"time\":\"{}\"", modified_field(meta))?;
        }
        Ok(())
    }

    // -- xml -------------------------------------------------------------

    fn xml_root(path: &str, opts: &Options, st: &mut State, out: &mut dyn Write) -> io::Result<()> {
        let root = Path::new(path);
        let (meta, link) = match Self::root_meta(root) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("tree: {}: {}", path, e);
                st.failed = true;
                return Ok(());
            }
        };
        Self::count_root(&meta, st);

        let tag = root_type_name(&meta, &link);
        write!(out, "  <{} name=\"", tag)?;
        write_xml_escaped(out, path.as_bytes())?;
        write!(out, "\"")?;
        Self::xml_link(link.as_deref(), out)?;
        Self::xml_meta(&meta, opts, out)?;

        if !meta.is_dir() {
            writeln!(out, "></{}>", tag)?;
            return Ok(());
        }

        match Self::read_children(root, opts, st) {
            Ok(children) => {
                writeln!(out, ">")?;
                Self::walk_xml(&children, 4, 1, opts, st, out)?;
                writeln!(out, "  </{}>", tag)?;
            }
            Err(_) => {
                writeln!(out, " error=\"opening dir\"></{}>", tag)?;
                st.failed = true;
            }
        }
        Ok(())
    }

    fn walk_xml(
        children: &[Entry],
        indent: usize,
        depth: usize,
        opts: &Options,
        st: &mut State,
        out: &mut dyn Write,
    ) -> io::Result<()> {
        for entry in children {
            Self::count(entry, st);

            let tag = type_name(&entry.meta);
            write!(out, "{:1$}", "", indent)?;
            write!(out, "<{} name=\"", tag)?;
            write_xml_escaped(out, &os_bytes(&entry.name))?;
            write!(out, "\"")?;
            Self::xml_link(entry.link_target.as_deref(), out)?;
            Self::xml_meta(&entry.meta, opts, out)?;

            match Self::descend(entry, depth, opts, st) {
                Some(Err(_)) => {
                    writeln!(out, " error=\"opening dir\"></{}>", tag)?;
                    st.failed = true;
                }
                Some(Ok(grandchildren)) => {
                    writeln!(out, ">")?;
                    Self::walk_xml(&grandchildren, indent + 2, depth + 1, opts, st, out)?;
                    write!(out, "{:1$}", "", indent)?;
                    writeln!(out, "</{}>", tag)?;
                }
                None => writeln!(out, "></{}>", tag)?,
            }
        }
        Ok(())
    }

    fn xml_link(target: Option<&Path>, out: &mut dyn Write) -> io::Result<()> {
        let Some(target) = target else {
            return Ok(());
        };
        write!(out, " target=\"")?;
        write_xml_escaped(out, &os_bytes(target.as_os_str()))?;
        write!(out, "\"")
    }

    fn xml_meta(meta: &Metadata, opts: &Options, out: &mut dyn Write) -> io::Result<()> {
        if opts.perms {
            write!(
                out,
                " mode=\"{}\" prot=\"{}\"",
                mode_field(meta),
                permission_field(meta)
            )?;
        }
        if opts.user {
            write!(out, " user=\"")?;
            write_xml_escaped(out, owner_name(meta).as_bytes())?;
            write!(out, "\"")?;
        }
        if opts.group {
            write!(out, " group=\"")?;
            write_xml_escaped(out, group_name(meta).as_bytes())?;
            write!(out, "\"")?;
        }
        if opts.size {
            // Raw bytes even under `-h`: upstream's `xml_fillinfo()` has no
            // human-readable branch, and consumers parse this as a number.
            write!(out, " size=\"{}\"", meta.len())?;
        }
        if opts.mtime {
            write!(out, " time=\"{}\"", modified_field(meta))?;
        }
        Ok(())
    }

    // -- html ------------------------------------------------------------

    fn html_header(out: &mut dyn Write) -> io::Result<()> {
        writeln!(out, "<!DOCTYPE html>")?;
        writeln!(out, "<html>")?;
        writeln!(out, "<head>")?;
        writeln!(out, "<meta charset=\"utf-8\">")?;
        writeln!(out, "<title>Directory Tree</title>")?;
        writeln!(out, "</head>")?;
        writeln!(out, "<body>")
    }

    fn html_root(
        path: &str,
        opts: &Options,
        st: &mut State,
        out: &mut dyn Write,
    ) -> io::Result<()> {
        let root = Path::new(path);
        let (meta, _) = match Self::root_meta(root) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("tree: {}: {}", path, e);
                st.failed = true;
                return Ok(());
            }
        };
        Self::count_root(&meta, st);

        let base = opts.html_base.trim_end_matches('/');
        writeln!(out, "<p>")?;
        Self::html_meta(&meta, opts, out)?;
        write!(out, "<a href=\"")?;
        write_xml_escaped(out, base.as_bytes())?;
        write!(out, "\">")?;
        write_xml_escaped(out, path.as_bytes())?;
        write!(out, "</a>")?;
        if opts.classify {
            Self::write_marker(&meta, root, out)?;
        }
        writeln!(out, "<br>")?;

        if meta.is_dir() {
            match Self::read_children(root, opts, st) {
                Ok(children) => Self::walk_html(&children, b"", 1, 1, opts, st, out)?,
                Err(_) => {
                    writeln!(out, "[error opening dir]<br>")?;
                    st.failed = true;
                }
            }
        }
        writeln!(out, "</p>")
    }

    fn walk_html(
        children: &[Entry],
        rel: &[u8],
        indent: usize,
        depth: usize,
        opts: &Options,
        st: &mut State,
        out: &mut dyn Write,
    ) -> io::Result<()> {
        let base = opts.html_base.trim_end_matches('/');
        for entry in children {
            Self::count(entry, st);

            let name = os_bytes(&entry.name);
            let child_rel = if rel.is_empty() {
                name.to_vec()
            } else {
                let mut joined = Vec::with_capacity(rel.len() + 1 + name.len());
                joined.extend_from_slice(rel);
                joined.push(b'/');
                joined.extend_from_slice(&name);
                joined
            };

            for _ in 0..indent {
                write!(out, "&nbsp;&nbsp;&nbsp;&nbsp;")?;
            }
            Self::html_meta(&entry.meta, opts, out)?;
            // The href is percent-encoded from the raw bytes, so it survives a
            // name that is not valid UTF-8 and needs no further XML escaping;
            // the link text still does.
            write!(out, "<a href=\"")?;
            write_xml_escaped(out, base.as_bytes())?;
            out.write_all(b"/")?;
            write_url_encoded(out, &child_rel)?;
            write!(out, "\">")?;
            write_xml_escaped(out, &name)?;
            write!(out, "</a>")?;
            if opts.classify {
                Self::write_marker(&entry.meta, &entry.path, out)?;
            }

            match Self::descend(entry, depth, opts, st) {
                Some(Err(_)) => {
                    writeln!(out, " [error opening dir]<br>")?;
                    st.failed = true;
                }
                Some(Ok(grandchildren)) => {
                    writeln!(out, "<br>")?;
                    Self::walk_html(
                        &grandchildren,
                        &child_rel,
                        indent + 1,
                        depth + 1,
                        opts,
                        st,
                        out,
                    )?;
                }
                None => writeln!(out, "<br>")?,
            }
        }
        Ok(())
    }

    // -- shared traversal -------------------------------------------------

    fn count(entry: &Entry, st: &mut State) {
        if entry.is_dir {
            st.dirs += 1;
        } else {
            st.files += 1;
        }
    }

    /// Count a path named on the command line. Upstream counts the root itself
    /// (`emit_tree()` bumps the directory total once `listdir()` returns), so
    /// leaving it out made every report one directory short.
    ///
    /// Upstream skips that bump for an empty or unreadable root, and charges an
    /// unreadable root to the *file* total instead; neither quirk is copied
    /// here — a root is worth one entry in the report whatever it holds.
    fn count_root(meta: &Metadata, st: &mut State) {
        if meta.is_dir() {
            st.dirs += 1;
        } else {
            st.files += 1;
        }
    }

    /// Read a directory's children ahead of printing its own line, so an
    /// unreadable directory can be flagged inline. Returns `None` when the entry
    /// is not a directory or the depth limit stops the descent.
    fn descend(
        entry: &Entry,
        depth: usize,
        opts: &Options,
        st: &mut State,
    ) -> Option<io::Result<Vec<Entry>>> {
        // `is_dir` looks through a link so the counters and `--dirsfirst` agree
        // with upstream, but the walk still stops at one: descending would
        // repeat whole subtrees and could loop.
        if !entry.is_dir || entry.is_symlink() {
            return None;
        }
        if let Some(max) = opts.max_depth {
            if depth >= max {
                return None;
            }
        }
        Some(Self::read_children(&entry.path, opts, st))
    }

    /// Metadata for a path named on the command line. Unlike entries inside the
    /// tree, a root symlink is followed — `tree link-to-dir` should list the
    /// directory the way `ls link-to-dir` does — but the link target is still
    /// reported alongside the name.
    fn root_meta(root: &Path) -> io::Result<(Metadata, Option<PathBuf>)> {
        let link = match fs::symlink_metadata(root) {
            Ok(m) if m.file_type().is_symlink() => fs::read_link(root).ok(),
            _ => None,
        };
        Ok((fs::metadata(root)?, link))
    }

    fn read_children(dir: &Path, opts: &Options, st: &mut State) -> io::Result<Vec<Entry>> {
        let mut entries = Vec::new();

        for item in fs::read_dir(dir)? {
            let item = item?;
            let name = item.file_name();

            if !opts.all && os_bytes(&name).first() == Some(&b'.') {
                continue;
            }
            if matches_any(&opts.exclude, &name) {
                continue;
            }

            let path = item.path();
            // The scratch file `-o` is writing into is not part of the tree.
            if opts.staged_output.as_deref() == Some(path.as_path()) {
                continue;
            }

            let meta = match fs::symlink_metadata(&path) {
                Ok(m) => m,
                // An entry that vanished mid-walk, or one we may not stat, is
                // dropped from the listing — but never silently.
                Err(e) => {
                    eprintln!("tree: {}: {}", path.display(), e);
                    st.failed = true;
                    continue;
                }
            };
            let file_type = meta.file_type();
            // Upstream's `getinfo()` classifies with `stat()`, so a link to a
            // directory counts as one, survives `-d` and sorts with the
            // directories — even though the walk will not follow it.
            let is_dir = if file_type.is_symlink() {
                fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false)
            } else {
                file_type.is_dir()
            };

            if opts.dirs_only && !is_dir {
                continue;
            }
            // -P filters files only, so the tree keeps its shape. The exemption
            // is decided on the `lstat` type, as upstream does: a link to a
            // directory is still filtered.
            if !file_type.is_dir() && !opts.include.is_empty() && !matches_any(&opts.include, &name)
            {
                continue;
            }

            let link_target = if file_type.is_symlink() {
                fs::read_link(&path).ok()
            } else {
                None
            };

            entries.push(Entry {
                name,
                path,
                meta,
                is_dir,
                link_target,
            });
        }

        Self::sort_entries(&mut entries, opts);
        Ok(entries)
    }

    /// `--dirsfirst` outranks `-r`: reversing sorts within each group but never
    /// puts files ahead of directories.
    fn sort_entries(entries: &mut [Entry], opts: &Options) {
        entries.sort_unstable_by(|a, b| {
            if opts.dirs_first {
                match (a.is_dir, b.is_dir) {
                    (true, false) => return Ordering::Less,
                    (false, true) => return Ordering::Greater,
                    _ => {}
                }
            }

            let ord = if opts.time_sort {
                entry_mtime(a)
                    .cmp(&entry_mtime(b))
                    .then_with(|| a.name.cmp(&b.name))
            } else {
                a.name.cmp(&b.name)
            };

            if opts.reverse {
                ord.reverse()
            } else {
                ord
            }
        });
    }

    // -- text rendering helpers -------------------------------------------

    fn write_entry_name(entry: &Entry, opts: &Options, out: &mut dyn Write) -> io::Result<()> {
        let display = if opts.full_path {
            os_bytes(entry.path.as_os_str())
        } else {
            os_bytes(&entry.name)
        };

        Self::write_colored(&display, &entry.meta, &entry.path, opts, out)?;

        if opts.classify {
            Self::write_marker(&entry.meta, &entry.path, out)?;
        }
        if let Some(target) = &entry.link_target {
            out.write_all(b" -> ")?;
            out.write_all(&os_bytes(target.as_os_str()))?;
        }
        Ok(())
    }

    fn write_colored(
        text: &[u8],
        meta: &Metadata,
        path: &Path,
        opts: &Options,
        out: &mut dyn Write,
    ) -> io::Result<()> {
        match if opts.color {
            get_color(meta, path)
        } else {
            None
        } {
            Some(color) => {
                write!(out, "\x1b[{}m", color)?;
                out.write_all(text)?;
                out.write_all(b"\x1b[0m")
            }
            None => out.write_all(text),
        }
    }

    fn write_marker(meta: &Metadata, path: &Path, out: &mut dyn Write) -> io::Result<()> {
        match type_marker(meta, path) {
            Some(marker) => write!(out, "{}", marker),
            None => Ok(()),
        }
    }

    /// The bracketed metadata columns that precede a name, e.g.
    /// `[drwxr-xr-x root root 4096 Aug 15 12:00]  name`.
    fn write_meta(meta: &Metadata, opts: &Options, out: &mut dyn Write) -> io::Result<()> {
        if !opts.any_meta() {
            return Ok(());
        }

        write!(out, "[")?;
        let mut first = true;

        if opts.perms {
            write!(out, "{}", permission_field(meta))?;
            first = false;
        }
        if opts.user {
            write!(
                out,
                "{}{:<8}",
                if first { "" } else { " " },
                owner_name(meta)
            )?;
            first = false;
        }
        if opts.group {
            write!(
                out,
                "{}{:<8}",
                if first { "" } else { " " },
                group_name(meta)
            )?;
            first = false;
        }
        if opts.size {
            let size = if opts.human {
                human_size(meta.len(), false, true)
            } else {
                meta.len().to_string()
            };
            write!(out, "{}{:>8}", if first { "" } else { " " }, size)?;
            first = false;
        }
        if opts.mtime {
            write!(
                out,
                "{}{}",
                if first { "" } else { " " },
                modified_field(meta)
            )?;
        }

        write!(out, "]  ")
    }

    /// The same bracketed columns, escaped for HTML. The padding that lines the
    /// columns up has to survive as `&nbsp;`, since a browser collapses runs of
    /// plain spaces.
    fn html_meta(meta: &Metadata, opts: &Options, out: &mut dyn Write) -> io::Result<()> {
        if !opts.any_meta() {
            return Ok(());
        }
        let mut buf = Vec::new();
        Self::write_meta(meta, opts, &mut buf)?;
        for &byte in &buf {
            match byte {
                b' ' => out.write_all(b"&nbsp;")?,
                other => write_xml_escaped(out, &[other])?,
            }
        }
        Ok(())
    }
}

// -- names as bytes --------------------------------------------------------

/// The bytes a file name is actually made of.
#[cfg(unix)]
fn os_bytes(text: &OsStr) -> Cow<'_, [u8]> {
    Cow::Borrowed(text.as_bytes())
}

/// Windows file names are UTF-16 rather than bytes, so the lossy step cannot be
/// avoided there; on Unix the raw bytes reach the output untouched.
#[cfg(not(unix))]
fn os_bytes(text: &OsStr) -> Cow<'_, [u8]> {
    match text.to_string_lossy() {
        Cow::Borrowed(borrowed) => Cow::Borrowed(borrowed.as_bytes()),
        Cow::Owned(owned) => Cow::Owned(owned.into_bytes()),
    }
}

// -- metadata fields ------------------------------------------------------

/// The `type` of `-J` and the element name of `-X`, mirroring upstream's
/// `ftype[]` table. The classification comes from the `lstat` result, so a
/// symlink is a `link` whatever it points at.
#[cfg(unix)]
fn type_name(meta: &Metadata) -> &'static str {
    let file_type = meta.file_type();
    if file_type.is_dir() {
        "directory"
    } else if file_type.is_symlink() {
        "link"
    } else if file_type.is_char_device() {
        "char"
    } else if file_type.is_block_device() {
        "block"
    } else if file_type.is_socket() {
        "socket"
    } else if file_type.is_fifo() {
        "fifo"
    } else if file_type.is_file() {
        "file"
    } else {
        "unknown"
    }
}

#[cfg(not(unix))]
fn type_name(meta: &Metadata) -> &'static str {
    let file_type = meta.file_type();
    if file_type.is_dir() {
        "directory"
    } else if file_type.is_symlink() {
        "link"
    } else if file_type.is_file() {
        "file"
    } else {
        "unknown"
    }
}

/// `root_meta` follows a root symlink, so its metadata describes the target.
/// The serialized type still has to name the link itself.
fn root_type_name(meta: &Metadata, link: &Option<PathBuf>) -> &'static str {
    if link.is_some() {
        "link"
    } else {
        type_name(meta)
    }
}

/// The numeric half of `-p`: permission bits plus setuid/setgid/sticky, the
/// same mask upstream prints as `"mode"` next to the symbolic `"prot"`.
#[cfg(unix)]
fn mode_field(meta: &Metadata) -> String {
    format!("{:04o}", meta.mode() & 0o7777)
}

#[cfg(not(unix))]
fn mode_field(meta: &Metadata) -> String {
    if meta.permissions().readonly() {
        "0444".to_string()
    } else {
        "0666".to_string()
    }
}

#[cfg(unix)]
fn permission_field(meta: &Metadata) -> String {
    let file_type = meta.file_type();
    let kind = if file_type.is_dir() {
        'd'
    } else if file_type.is_symlink() {
        'l'
    } else if file_type.is_block_device() {
        'b'
    } else if file_type.is_char_device() {
        'c'
    } else if file_type.is_fifo() {
        'p'
    } else if file_type.is_socket() {
        's'
    } else {
        '-'
    };
    format!("{}{}", kind, format_permissions(meta.mode()))
}

#[cfg(not(unix))]
fn permission_field(meta: &Metadata) -> String {
    let kind = if meta.is_dir() { 'd' } else { '-' };
    if meta.permissions().readonly() {
        format!("{}r--r--r--", kind)
    } else {
        format!("{}rw-rw-rw-", kind)
    }
}

#[cfg(unix)]
fn owner_name(meta: &Metadata) -> String {
    let uid = meta.uid();
    get_username_by_uid(uid).unwrap_or_else(|| uid.to_string())
}

#[cfg(not(unix))]
fn owner_name(_meta: &Metadata) -> String {
    "-".to_string()
}

#[cfg(unix)]
fn group_name(meta: &Metadata) -> String {
    let gid = meta.gid();
    get_group_name_by_gid(gid).unwrap_or_else(|| gid.to_string())
}

#[cfg(not(unix))]
fn group_name(_meta: &Metadata) -> String {
    "-".to_string()
}

fn modified_field(meta: &Metadata) -> String {
    format_time(meta.modified().unwrap_or(SystemTime::UNIX_EPOCH))
}

/// Sort key for `-t`. Reads the already-cached `stat` buffer, so calling it from
/// the comparator costs no syscalls.
fn entry_mtime(entry: &Entry) -> SystemTime {
    entry.meta.modified().unwrap_or(SystemTime::UNIX_EPOCH)
}

#[cfg(unix)]
fn type_marker(meta: &Metadata, _path: &Path) -> Option<char> {
    let file_type = meta.file_type();
    if file_type.is_dir() {
        Some('/')
    } else if file_type.is_symlink() {
        Some('@')
    } else if file_type.is_fifo() {
        Some('|')
    } else if file_type.is_socket() {
        Some('=')
    } else if meta.mode() & 0o111 != 0 {
        Some('*')
    } else {
        None
    }
}

#[cfg(not(unix))]
fn type_marker(meta: &Metadata, path: &Path) -> Option<char> {
    let file_type = meta.file_type();
    if file_type.is_dir() {
        Some('/')
    } else if file_type.is_symlink() {
        Some('@')
    } else {
        let ext = path.extension()?.to_string_lossy().to_lowercase();
        match ext.as_str() {
            "exe" | "bat" | "cmd" | "com" => Some('*'),
            _ => None,
        }
    }
}

// -- account lookups (mirrors id.rs so both stay small and self-contained) ---

#[cfg(unix)]
fn c_char_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}

#[cfg(unix)]
fn get_username_by_uid(uid: u32) -> Option<String> {
    let _account_db_guard = lock_account_db();
    let ptr = unsafe { raw_getpwuid(uid) };
    if ptr.is_null() {
        return None;
    }
    unsafe {
        let pw_name = (*ptr).pw_name;
        if pw_name.is_null() {
            return None;
        }
        Some(c_char_to_string(pw_name))
    }
}

#[cfg(unix)]
fn get_group_name_by_gid(gid: u32) -> Option<String> {
    let _account_db_guard = lock_account_db();
    let ptr = unsafe { raw_getgrgid(gid) };
    if ptr.is_null() {
        return None;
    }
    unsafe {
        let gr_name = (*ptr).gr_name;
        if gr_name.is_null() {
            return None;
        }
        Some(c_char_to_string(gr_name))
    }
}

// -- formatting helpers (mirrors ls.rs) -------------------------------------

#[cfg(unix)]
fn format_permissions(mode: u32) -> String {
    let mut result = String::with_capacity(9);

    result.push(if mode & 0o400 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o200 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o100 != 0 {
        if mode & 0o4000 != 0 {
            's'
        } else {
            'x'
        }
    } else if mode & 0o4000 != 0 {
        'S'
    } else {
        '-'
    });

    result.push(if mode & 0o040 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o020 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o010 != 0 {
        if mode & 0o2000 != 0 {
            's'
        } else {
            'x'
        }
    } else if mode & 0o2000 != 0 {
        'S'
    } else {
        '-'
    });

    result.push(if mode & 0o004 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o002 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o001 != 0 {
        if mode & 0o1000 != 0 {
            't'
        } else {
            'x'
        }
    } else if mode & 0o1000 != 0 {
        'T'
    } else {
        '-'
    });

    result
}

/// Seconds since the epoch, read once: `-D` compares every entry against the
/// same "now" and a large tree would otherwise ask the clock per file.
fn now_secs() -> i64 {
    static NOW: OnceLock<i64> = OnceLock::new();
    *NOW.get_or_init(|| {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    })
}

/// `-D`, in the two shapes `ls -l` and upstream `tree` both use: a clock time
/// for something recent, a year once the timestamp is more than six months old
/// or lies in the future.
///
/// The calendar is UTC rather than local time, matching `ls.rs`; converting
/// would mean carrying a timezone database or calling into libc.
fn format_time(time: SystemTime) -> String {
    const SIX_MONTHS: i64 = 6 * 31 * 86400;

    let duration = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs() as i64;
    let (year, month, day, hour, min) = unix_to_datetime(secs);

    let now = now_secs();
    if secs > now || secs + SIX_MONTHS < now {
        format!("{} {:>2}  {}", month_name(month), day, year)
    } else {
        format!("{} {:>2} {:02}:{:02}", month_name(month), day, hour, min)
    }
}

fn unix_to_datetime(secs: i64) -> (i32, u32, u32, u32, u32) {
    const DAYS_PER_YEAR: i64 = 365;
    const DAYS_PER_LEAP: i64 = 366;

    let mut days = secs / 86400;
    let mut year = 1970;

    loop {
        let days_in_year = if is_leap_year(year) {
            DAYS_PER_LEAP
        } else {
            DAYS_PER_YEAR
        };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0;
    for (i, &dim) in days_in_months.iter().enumerate() {
        if days < dim {
            month = i + 1;
            break;
        }
        days -= dim;
    }

    let day = days + 1;
    let time_of_day = secs % 86400;
    let hour = (time_of_day / 3600) as u32;
    let min = ((time_of_day % 3600) / 60) as u32;

    (year, month as u32, day as u32, hour, min)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

fn get_color(meta: &Metadata, path: &Path) -> Option<&'static str> {
    let file_type = meta.file_type();

    if file_type.is_dir() {
        return Some("1;34");
    }
    if file_type.is_symlink() {
        return Some("1;36");
    }

    if meta.is_file() {
        #[cfg(unix)]
        {
            if meta.mode() & 0o111 != 0 {
                return Some("1;32");
            }
        }

        #[cfg(not(unix))]
        {
            if !meta.permissions().readonly() {
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    match ext.as_str() {
                        "exe" | "bat" | "cmd" | "com" => return Some("1;32"),
                        _ => {}
                    }
                }
            }
        }

        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            match ext.as_str() {
                "tar" | "gz" | "bz2" | "xz" | "zip" | "rar" | "7z" | "tgz" => {
                    return Some("1;31");
                }
                _ => {}
            }
        }
    }

    None
}

// -- pattern matching -------------------------------------------------------

/// True when any of `patterns` matches `name`.
fn matches_any(patterns: &[String], name: &OsStr) -> bool {
    let name = os_bytes(name);
    patterns
        .iter()
        .any(|pattern| pattern_match(pattern.as_bytes(), &name))
}

/// Matches one `-I`/`-P` pattern against a single name, byte for byte.
///
/// Deliberately richer than the `glob_match` in `find.rs`: `tree` patterns are
/// documented to take `|` alternation and `[...]` classes and scripts pass both,
/// so the two stay separate copies rather than growing a shared layer.
///
/// Unlike upstream's `patmatch()`, the alternation is split outside brackets
/// only — upstream's `strchr` cuts `[a|b]` in half, hits its own syntax error
/// and then reports that error as a match against everything.
fn pattern_match(pattern: &[u8], text: &[u8]) -> bool {
    split_alternatives(pattern)
        .into_iter()
        .any(|alternative| match_here(alternative, text))
}

fn split_alternatives(pattern: &[u8]) -> Vec<&[u8]> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut idx = 0;
    let mut in_class = false;

    while idx < pattern.len() {
        match pattern[idx] {
            b'\\' => idx += 1,
            b'[' if !in_class => in_class = true,
            b']' if in_class => in_class = false,
            b'|' if !in_class => {
                parts.push(&pattern[start..idx]);
                start = idx + 1;
            }
            _ => {}
        }
        idx += 1;
    }

    parts.push(&pattern[start..]);
    parts
}

fn match_here(pattern: &[u8], text: &[u8]) -> bool {
    let Some(&first) = pattern.first() else {
        return text.is_empty();
    };

    match first {
        b'*' => {
            let rest = &pattern[1..];
            (0..=text.len()).any(|split| match_here(rest, &text[split..]))
        }
        b'?' => !text.is_empty() && match_here(&pattern[1..], &text[1..]),
        b'[' => match match_class(&pattern[1..], text.first().copied()) {
            Some((true, consumed)) => match_here(&pattern[1 + consumed..], &text[1..]),
            Some((false, _)) => false,
            // A class that is never closed is a literal `[`.
            None => !text.is_empty() && text[0] == b'[' && match_here(&pattern[1..], &text[1..]),
        },
        b'\\' if pattern.len() > 1 => {
            !text.is_empty() && text[0] == pattern[1] && match_here(&pattern[2..], &text[1..])
        }
        literal => !text.is_empty() && text[0] == literal && match_here(&pattern[1..], &text[1..]),
    }
}

/// Scans a `[...]` class starting just past the `[`. Returns whether
/// `candidate` falls inside it and how many pattern bytes the class spans
/// (including the closing `]`), or `None` if it is never closed.
fn match_class(pattern: &[u8], candidate: Option<u8>) -> Option<(bool, usize)> {
    let mut idx = 0;
    let negated = pattern.first() == Some(&b'^');
    if negated {
        idx += 1;
    }

    let mut matched = false;
    // A `]` in the leading position is a literal, the way POSIX classes work.
    let mut leading = true;

    while idx < pattern.len() {
        if pattern[idx] == b']' && !leading {
            let inside = if negated { !matched } else { matched };
            return Some((candidate.is_some() && inside, idx + 1));
        }
        leading = false;

        if pattern[idx] == b'\\' && idx + 1 < pattern.len() {
            idx += 1;
        }
        let low = pattern[idx];
        idx += 1;

        // `a-z` is a range unless the `-` is the last member of the class.
        if idx + 1 < pattern.len() && pattern[idx] == b'-' && pattern[idx + 1] != b']' {
            idx += 1;
            if pattern[idx] == b'\\' && idx + 1 < pattern.len() {
                idx += 1;
            }
            let high = pattern[idx];
            idx += 1;
            if let Some(byte) = candidate {
                if byte >= low && byte <= high {
                    matched = true;
                }
            }
        } else if candidate == Some(low) {
            matched = true;
        }
    }

    None
}

// -- escaping ---------------------------------------------------------------

/// RFC 8259 string escaping, applied to raw bytes. Bytes at or above 0x20 pass
/// through untouched, so a name that is not valid UTF-8 survives the round trip
/// the same way it does through upstream's `json_encode()`.
fn write_json_escaped(out: &mut dyn Write, text: &[u8]) -> io::Result<()> {
    for &byte in text {
        match byte {
            b'"' => out.write_all(b"\\\"")?,
            b'\\' => out.write_all(b"\\\\")?,
            b'\n' => out.write_all(b"\\n")?,
            b'\r' => out.write_all(b"\\r")?,
            b'\t' => out.write_all(b"\\t")?,
            control if control < 0x20 => write!(out, "\\u{:04x}", control)?,
            other => out.write_all(&[other])?,
        }
    }
    Ok(())
}

/// The five XML entities, also used for HTML.
///
/// C0 control characters have no legal spelling in XML 1.0 — not even as a
/// numeric reference — so they become `?` rather than making the document
/// unparseable. Upstream passes them through and produces XML no parser will
/// read; `?` is the substitution its own `-q` flag uses for the same bytes.
fn write_xml_escaped(out: &mut dyn Write, text: &[u8]) -> io::Result<()> {
    for &byte in text {
        match byte {
            b'&' => out.write_all(b"&amp;")?,
            b'<' => out.write_all(b"&lt;")?,
            b'>' => out.write_all(b"&gt;")?,
            b'"' => out.write_all(b"&quot;")?,
            b'\'' => out.write_all(b"&apos;")?,
            control if control < 0x20 && !matches!(control, b'\t' | b'\n' | b'\r') => {
                out.write_all(b"?")?
            }
            other => out.write_all(&[other])?,
        }
    }
    Ok(())
}

/// Percent-encode a relative path for use inside an `href`. Everything outside
/// the RFC 3986 unreserved set is escaped, so a name holding `#`, `?`, `%`, a
/// space or a byte that is not valid UTF-8 still produces a link that resolves
/// to that file. `/` is kept as-is, since the input is a path rather than a
/// single component.
fn write_url_encoded(out: &mut dyn Write, text: &[u8]) -> io::Result<()> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    for &byte in text {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.write_all(&[byte])?
            }
            other => out.write_all(&[
                b'%',
                HEX[(other >> 4) as usize],
                HEX[(other & 0xf) as usize],
            ])?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn escaped_bytes(writer: fn(&mut dyn Write, &[u8]) -> io::Result<()>, text: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        writer(&mut buf, text).unwrap();
        buf
    }

    fn escaped(writer: fn(&mut dyn Write, &[u8]) -> io::Result<()>, text: &[u8]) -> String {
        String::from_utf8_lossy(&escaped_bytes(writer, text)).into_owned()
    }

    #[test]
    fn json_escape_handles_quotes_and_controls() {
        assert_eq!(escaped(write_json_escaped, br#"a"b\c"#), r#"a\"b\\c"#);
        assert_eq!(escaped(write_json_escaped, b"a\nb\tc"), "a\\nb\\tc");
        assert_eq!(escaped(write_json_escaped, b"a\x01b"), "a\\u0001b");
        assert_eq!(escaped(write_json_escaped, b"plain"), "plain");
        // Bytes that are not valid UTF-8 pass through rather than collapsing
        // into a replacement character, so the name still identifies the file.
        assert_eq!(escaped_bytes(write_json_escaped, b"bad\xff"), b"bad\xff");
    }

    #[test]
    fn xml_escape_handles_entities_and_controls() {
        assert_eq!(escaped(write_xml_escaped, b"a&b<c>d"), "a&amp;b&lt;c&gt;d");
        assert_eq!(
            escaped(write_xml_escaped, b"say \"hi\""),
            "say &quot;hi&quot;"
        );
        assert_eq!(escaped(write_xml_escaped, b"it's"), "it&apos;s");
        // XML 1.0 cannot represent these at all, so they must not reach the
        // document verbatim.
        assert_eq!(escaped(write_xml_escaped, b"ctl\x01name"), "ctl?name");
        assert_eq!(escaped(write_xml_escaped, b"tab\there"), "tab\there");
    }

    #[test]
    fn url_encode_escapes_everything_but_unreserved() {
        assert_eq!(escaped(write_url_encoded, b"a/b/c.txt"), "a/b/c.txt");
        assert_eq!(escaped(write_url_encoded, b"-_.~"), "-_.~");
        assert_eq!(
            escaped(write_url_encoded, b"we ird#name?q"),
            "we%20ird%23name%3Fq"
        );
        assert_eq!(escaped(write_url_encoded, b"100%"), "100%25");
        // Multi-byte characters are encoded one UTF-8 byte at a time.
        assert_eq!(escaped(write_url_encoded, "é".as_bytes()), "%C3%A9");
        // Nothing an XML attribute would have to escape survives.
        assert_eq!(escaped(write_url_encoded, b"a&b\"c<d>"), "a%26b%22c%3Cd%3E");
        // A byte that is not valid UTF-8 still produces a usable link.
        assert_eq!(escaped(write_url_encoded, b"bad\xff"), "bad%FF");
    }

    #[test]
    fn pattern_match_handles_wildcards() {
        assert!(pattern_match(b"*.rs", b"main.rs"));
        assert!(!pattern_match(b"*.rs", b"main.md"));
        assert!(pattern_match(b"?.txt", b"a.txt"));
        assert!(!pattern_match(b"?.txt", b"ab.txt"));
        assert!(pattern_match(b"*", b""));
        assert!(pattern_match(b"a*b*c", b"axxbyyc"));
        assert!(!pattern_match(b"a*b*c", b"axxbyy"));
    }

    #[test]
    fn pattern_match_handles_alternation() {
        assert!(pattern_match(b"old|new", b"old"));
        assert!(pattern_match(b"old|new", b"new"));
        assert!(!pattern_match(b"old|new", b"other"));
        assert!(pattern_match(b"*.rs|*.md", b"notes.md"));
        // A `|` inside a class is part of the class, not a split point.
        assert!(pattern_match(b"[a|b]", b"|"));
        assert!(!pattern_match(b"[a|b]", b"c"));
    }

    #[test]
    fn pattern_match_handles_character_classes() {
        assert!(pattern_match(b"[ab]*", b"apple"));
        assert!(pattern_match(b"[ab]*", b"banana"));
        assert!(!pattern_match(b"[ab]*", b"cherry"));
        assert!(pattern_match(b"[^ab]*", b"cherry"));
        assert!(!pattern_match(b"[^ab]*", b"apple"));
        assert!(pattern_match(b"[a-f]oo", b"doo"));
        assert!(!pattern_match(b"[a-f]oo", b"zoo"));
        // A `-` at the end of the class is a literal.
        assert!(pattern_match(b"[a-]", b"-"));
        // An unterminated class is a literal bracket.
        assert!(pattern_match(b"[abc", b"[abc"));
        // An empty name never falls inside a class.
        assert!(!pattern_match(b"[abc]", b""));
    }

    #[test]
    fn pattern_match_handles_escapes() {
        assert!(pattern_match(br"a\*b", b"a*b"));
        assert!(!pattern_match(br"a\*b", b"axxb"));
        assert!(pattern_match(br"a\?b", b"a?b"));
        assert!(pattern_match(br"[\]]", b"]"));
    }
}
