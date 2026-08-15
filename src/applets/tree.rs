//! Tree-style directory listing.
//!
//! Mirrors the classic `tree(1)` layout: every directory level adds a prefix of
//! connector glyphs so the hierarchy stays readable, and the traversal caches
//! each entry's metadata so a file is only `stat`ed once no matter how many of
//! the `-s`/`-p`/`-u`/`-g`/`-D` columns are enabled.

#[cfg(unix)]
use crate::core::unix_ffi::{lock_account_db, raw_getgrgid, raw_getpwuid};
use crate::core::{human_size, Applet};
use std::cmp::Ordering;
#[cfg(unix)]
use std::ffi::{c_char, CStr};
use std::fs::{self, Metadata};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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
    include: Option<String>,
    exclude: Option<String>,

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
    name: String,
    path: PathBuf,
    meta: Metadata,
    is_dir: bool,
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
                        match args[i].to_ascii_uppercase().as_str() {
                            "ASCII" => opts.ascii = true,
                            "UTF-8" | "UTF8" => opts.ascii = false,
                            _ => {
                                eprintln!("tree: unsupported charset: {}", args[i]);
                                return Ok(1);
                            }
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
                    'n' => opts.color = false,
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
                            'I' => opts.exclude = Some(value),
                            'P' => opts.include = Some(value),
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

        let mut out: Box<dyn Write> = match &opts.output {
            Some(file) => match fs::File::create(file) {
                Ok(f) => Box::new(io::BufWriter::new(f)),
                Err(e) => {
                    eprintln!("tree: cannot open output file '{}': {}", file, e);
                    return Ok(1);
                }
            },
            None => Box::new(io::BufWriter::new(io::stdout())),
        };

        let code = Self::render(&paths, &opts, &mut *out)?;
        out.flush()?;
        Ok(code)
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
        println!("  -I PATTERN      Do not list entries matching PATTERN");
        println!("  -P PATTERN      List only files matching PATTERN");
        println!("  -i              Print entries without indentation or connector lines");
        println!();
        println!("File information:");
        println!("  -s              Print the size of each file in bytes");
        println!("  -h              Print the size of each file in human readable form");
        println!("  -p              Print the permissions of each file");
        println!("  -u              Print the owner name of each file");
        println!("  -g              Print the group name of each file");
        println!("  -D              Print the last modification time of each file");
        println!("  -F              Append a type indicator (/, *, @, |, =) to each entry");
        println!();
        println!("Sorting:");
        println!("  --dirsfirst     List directories before files");
        println!("  -r              Sort in reverse order");
        println!("  -t              Sort by last modification time");
        println!();
        println!("Output:");
        println!("  -C              Colorize the output");
        println!("  -n              Do not colorize the output (default)");
        println!("  -o FILE         Write the output to FILE instead of standard output");
        println!("  -J              Print the tree as JSON");
        println!("  -X              Print the tree as XML");
        println!("  -H BASE         Print the tree as HTML, using BASE as the link prefix");
        println!("  --charset SET   Line-drawing character set: UTF-8 (default) or ASCII");
        println!("  --noreport      Omit the file and directory count at the end");
        println!();
        println!("An option that takes a value accepts it attached (-L2) or separate (-L 2).");
    }
}

impl TreeApplet {
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
                    writeln!(
                        out,
                        "  {{\"type\":\"report\",\"directories\":{},\"files\":{}}}",
                        st.dirs, st.files
                    )?;
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
                    writeln!(out, "    <files>{}</files>", st.files)?;
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

        Self::write_meta(&meta, opts, out)?;
        Self::write_colored(path, &meta, root, opts, out)?;
        if opts.classify {
            Self::write_marker(&meta, root, out)?;
        }
        if let Some(target) = &link {
            write!(out, " -> {}", target.display())?;
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
        let (meta, _) = match Self::root_meta(root) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("tree: {}: {}", path, e);
                st.failed = true;
                return Ok(false);
            }
        };

        if separator {
            writeln!(out, ",")?;
        }

        let kind = if meta.is_dir() { "directory" } else { "file" };
        write!(
            out,
            "  {{\"type\":\"{}\",\"name\":\"{}\"",
            kind,
            json_escape(path)
        )?;
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

            let kind = if entry.is_dir { "directory" } else { "file" };
            write!(out, "{:1$}", "", indent)?;
            write!(
                out,
                "{{\"type\":\"{}\",\"name\":\"{}\"",
                kind,
                json_escape(&entry.name)
            )?;
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

    fn json_meta(meta: &Metadata, opts: &Options, out: &mut dyn Write) -> io::Result<()> {
        if opts.size {
            write!(out, ",\"size\":{}", meta.len())?;
        }
        if opts.perms {
            write!(out, ",\"mode\":\"{}\"", permission_field(meta))?;
        }
        if opts.user {
            write!(out, ",\"user\":\"{}\"", json_escape(&owner_name(meta)))?;
        }
        if opts.group {
            write!(out, ",\"group\":\"{}\"", json_escape(&group_name(meta)))?;
        }
        if opts.mtime {
            write!(out, ",\"time\":\"{}\"", modified_field(meta))?;
        }
        Ok(())
    }

    // -- xml -------------------------------------------------------------

    fn xml_root(path: &str, opts: &Options, st: &mut State, out: &mut dyn Write) -> io::Result<()> {
        let root = Path::new(path);
        let (meta, _) = match Self::root_meta(root) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("tree: {}: {}", path, e);
                st.failed = true;
                return Ok(());
            }
        };

        let tag = if meta.is_dir() { "directory" } else { "file" };
        write!(out, "  <{} name=\"{}\"", tag, xml_escape(path))?;
        Self::xml_meta(&meta, opts, out)?;

        if !meta.is_dir() {
            writeln!(out, "></{}>", tag)?;
            return Ok(());
        }

        match Self::read_children(root, opts, st) {
            Ok(children) => {
                writeln!(out, ">")?;
                Self::walk_xml(&children, 4, 1, opts, st, out)?;
                writeln!(out, "  </directory>")?;
            }
            Err(_) => {
                writeln!(out, " error=\"opening dir\"></directory>")?;
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

            let tag = if entry.is_dir { "directory" } else { "file" };
            write!(out, "{:1$}", "", indent)?;
            write!(out, "<{} name=\"{}\"", tag, xml_escape(&entry.name))?;
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

    fn xml_meta(meta: &Metadata, opts: &Options, out: &mut dyn Write) -> io::Result<()> {
        if opts.size {
            write!(out, " size=\"{}\"", meta.len())?;
        }
        if opts.perms {
            write!(out, " mode=\"{}\"", permission_field(meta))?;
        }
        if opts.user {
            write!(out, " user=\"{}\"", xml_escape(&owner_name(meta)))?;
        }
        if opts.group {
            write!(out, " group=\"{}\"", xml_escape(&group_name(meta)))?;
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

        let base = opts.html_base.trim_end_matches('/');
        writeln!(out, "<p>")?;
        Self::html_meta(&meta, opts, out)?;
        write!(
            out,
            "<a href=\"{}\">{}</a>",
            xml_escape(base),
            xml_escape(path)
        )?;
        if opts.classify {
            Self::write_marker(&meta, root, out)?;
        }
        writeln!(out, "<br>")?;

        if meta.is_dir() {
            match Self::read_children(root, opts, st) {
                Ok(children) => Self::walk_html(&children, "", 1, 1, opts, st, out)?,
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
        rel: &str,
        indent: usize,
        depth: usize,
        opts: &Options,
        st: &mut State,
        out: &mut dyn Write,
    ) -> io::Result<()> {
        let base = opts.html_base.trim_end_matches('/');
        for entry in children {
            Self::count(entry, st);

            let child_rel = if rel.is_empty() {
                entry.name.clone()
            } else {
                format!("{}/{}", rel, entry.name)
            };

            for _ in 0..indent {
                write!(out, "&nbsp;&nbsp;&nbsp;&nbsp;")?;
            }
            Self::html_meta(&entry.meta, opts, out)?;
            // `url_encode` only ever emits unreserved characters and `/`, so the
            // href needs no further XML escaping; the link text still does.
            write!(
                out,
                "<a href=\"{}/{}\">{}</a>",
                xml_escape(base),
                url_encode(&child_rel),
                xml_escape(&entry.name)
            )?;
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

    /// Read a directory's children ahead of printing its own line, so an
    /// unreadable directory can be flagged inline. Returns `None` when the entry
    /// is not a directory or the depth limit stops the descent.
    fn descend(
        entry: &Entry,
        depth: usize,
        opts: &Options,
        st: &mut State,
    ) -> Option<io::Result<Vec<Entry>>> {
        if !entry.is_dir {
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
            let name = item.file_name().to_string_lossy().into_owned();

            if !opts.all && name.starts_with('.') {
                continue;
            }
            if let Some(pattern) = &opts.exclude {
                if glob_match(pattern, &name) {
                    continue;
                }
            }

            let path = item.path();
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
            let is_dir = meta.is_dir();

            if opts.dirs_only && !is_dir {
                continue;
            }
            // -P filters files only; directories stay so the tree keeps its shape.
            if let Some(pattern) = &opts.include {
                if !is_dir && !glob_match(pattern, &name) {
                    continue;
                }
            }

            entries.push(Entry {
                name,
                path,
                meta,
                is_dir,
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
            entry.path.display().to_string()
        } else {
            entry.name.clone()
        };

        Self::write_colored(&display, &entry.meta, &entry.path, opts, out)?;

        if opts.classify {
            Self::write_marker(&entry.meta, &entry.path, out)?;
        }
        if entry.meta.file_type().is_symlink() {
            if let Ok(target) = fs::read_link(&entry.path) {
                write!(out, " -> {}", target.display())?;
            }
        }
        Ok(())
    }

    fn write_colored(
        text: &str,
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
            Some(color) => write!(out, "\x1b[{}m{}\x1b[0m", color, text),
            None => write!(out, "{}", text),
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
        // Only the account names can be non-ASCII, and those came out of the
        // passwd database as UTF-8; lossy conversion keeps this infallible.
        let text = xml_escape(&String::from_utf8_lossy(&buf));
        write!(out, "{}", text.replace(' ', "&nbsp;"))
    }
}

// -- metadata fields ------------------------------------------------------

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

fn format_time(time: SystemTime) -> String {
    let duration = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs() as i64;
    let (_year, month, day, hour, min) = unix_to_datetime(secs);

    format!("{} {:>2} {:02}:{:02}", month_name(month), day, hour, min)
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

// -- pattern matching (mirrors find.rs) -------------------------------------

fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    glob_match_inner(&pattern, &text)
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

// -- escaping ---------------------------------------------------------------

fn json_escape(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if (c as u32) < 0x20 => result.push_str(&format!("\\u{:04x}", c as u32)),
            c => result.push(c),
        }
    }
    result
}

/// Also used for HTML, which needs the same five entities.
fn xml_escape(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            c => result.push(c),
        }
    }
    result
}

/// Percent-encode a relative path for use inside an `href`. Everything outside
/// the RFC 3986 unreserved set is escaped, so a name holding `#`, `?`, `%` or a
/// space still produces a link that resolves to that file. `/` is kept as-is,
/// since the input is a path rather than a single component.
fn url_encode(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                result.push(byte as char)
            }
            _ => {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                result.push('%');
                result.push(HEX[(byte >> 4) as usize] as char);
                result.push(HEX[(byte & 0xf) as usize] as char);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escape_handles_quotes_and_controls() {
        assert_eq!(json_escape(r#"a"b\c"#), r#"a\"b\\c"#);
        assert_eq!(json_escape("a\nb\tc"), "a\\nb\\tc");
        assert_eq!(json_escape("a\u{1}b"), "a\\u0001b");
        assert_eq!(json_escape("plain"), "plain");
    }

    #[test]
    fn xml_escape_handles_entities() {
        assert_eq!(xml_escape("a&b<c>d"), "a&amp;b&lt;c&gt;d");
        assert_eq!(xml_escape("say \"hi\""), "say &quot;hi&quot;");
        assert_eq!(xml_escape("it's"), "it&apos;s");
    }

    #[test]
    fn url_encode_escapes_everything_but_unreserved() {
        assert_eq!(url_encode("a/b/c.txt"), "a/b/c.txt");
        assert_eq!(url_encode("-_.~"), "-_.~");
        assert_eq!(url_encode("we ird#name?q"), "we%20ird%23name%3Fq");
        assert_eq!(url_encode("100%"), "100%25");
        // Multi-byte characters are encoded one UTF-8 byte at a time.
        assert_eq!(url_encode("é"), "%C3%A9");
        // Nothing an XML attribute would have to escape survives.
        assert_eq!(url_encode("a&b\"c<d>"), "a%26b%22c%3Cd%3E");
    }
}
