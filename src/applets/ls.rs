use crate::core::{human_size, Applet};
use std::fs::{self, DirEntry, Metadata};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};

pub struct LsApplet;

impl Applet for LsApplet {
    fn name(&self) -> &'static str {
        "ls"
    }

    fn description(&self) -> &'static str {
        "List directory contents"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut show_all = false;
        let mut long_format = false;
        let mut human_readable = false;
        let mut use_color = false;
        let mut paths: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];
            match arg.as_str() {
                "-a" | "--all" => show_all = true,
                "-l" => long_format = true,
                "-h" | "--human-readable" => human_readable = true,
                "--color" | "--color=always" => use_color = true,
                "--color=auto" => use_color = Self::is_tty(),
                "--color=never" => use_color = false,
                "-lh" | "-hl" => {
                    long_format = true;
                    human_readable = true;
                }
                "-la" | "-al" => {
                    long_format = true;
                    show_all = true;
                }
                "-lah" | "-lha" | "-ahl" | "-alh" | "-hal" | "-hla" => {
                    long_format = true;
                    show_all = true;
                    human_readable = true;
                }
                _ if arg.starts_with('-') && arg.len() > 1 => {
                    for ch in arg[1..].chars() {
                        match ch {
                            'a' => show_all = true,
                            'l' => long_format = true,
                            'h' => human_readable = true,
                            _ => return Err(format!("ls: invalid option -- '{}'", ch).into()),
                        }
                    }
                }
                _ => paths.push(arg),
            }
            i += 1;
        }

        if paths.is_empty() {
            paths.push(".");
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();

        for (idx, path) in paths.iter().enumerate() {
            if paths.len() > 1 {
                if idx > 0 {
                    writeln!(out)?;
                }
                writeln!(out, "{}:", path)?;
            }

            if let Err(e) = Self::list_path(
                path,
                &mut out,
                show_all,
                long_format,
                human_readable,
                use_color,
            ) {
                eprintln!("ls: cannot access '{}': {}", path, e);
                return Ok(1);
            }
        }

        Ok(0)
    }

    fn help(&self) {
        println!("Usage: ls [OPTION]... [FILE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -a, --all             Do not ignore entries starting with .");
        println!("  -l                    Use a long listing format");
        println!("  -h, --human-readable  With -l, print sizes in human readable format");
        println!("  --color[=WHEN]        Colorize the output (auto, always, never)");
    }
}

impl LsApplet {
    #[cfg(unix)]
    fn is_tty() -> bool {
        use std::io::IsTerminal;
        std::io::stdout().is_terminal()
    }

    #[cfg(not(unix))]
    fn is_tty() -> bool {
        false
    }

    fn list_path(
        path: &str,
        out: &mut impl Write,
        show_all: bool,
        long_format: bool,
        human_readable: bool,
        use_color: bool,
    ) -> io::Result<()> {
        let path = Path::new(path);
        let metadata = fs::symlink_metadata(path)?;

        if !metadata.is_dir() {
            let entry = Self::create_entry_from_path(path)?;
            if long_format {
                Self::print_long(&entry, out, human_readable, use_color)?;
            } else {
                Self::print_short(&entry, out, use_color)?;
                writeln!(out)?;
            }
            return Ok(());
        }

        let mut entries: Vec<DirEntry> = fs::read_dir(path)?.collect::<Result<_, _>>()?;

        entries.retain(|e| {
            if show_all {
                true
            } else {
                !e.file_name().to_string_lossy().starts_with('.')
            }
        });

        entries.sort_unstable_by(|a, b| {
            a.file_name()
                .to_string_lossy()
                .cmp(&b.file_name().to_string_lossy())
        });

        if long_format {
            for entry in &entries {
                Self::print_long(entry, out, human_readable, use_color)?;
            }
        } else {
            for entry in &entries {
                Self::print_short(entry, out, use_color)?;
                write!(out, "  ")?;
            }
            if !entries.is_empty() {
                writeln!(out)?;
            }
        }

        Ok(())
    }

    fn create_entry_from_path(path: &Path) -> io::Result<DirEntry> {
        let parent = path.parent().unwrap_or(Path::new("."));
        let file_name = path.file_name().unwrap_or(path.as_os_str());

        for entry in fs::read_dir(parent)? {
            let entry = entry?;
            if entry.file_name() == file_name {
                return Ok(entry);
            }
        }

        Err(io::Error::new(io::ErrorKind::NotFound, "Entry not found"))
    }

    fn print_short(entry: &DirEntry, out: &mut impl Write, use_color: bool) -> io::Result<()> {
        let name = entry.file_name().to_string_lossy().to_string();
        let metadata = fs::symlink_metadata(entry.path())?;

        if use_color {
            let color = Self::get_color(&metadata, entry.path());
            if let Some(c) = color {
                write!(out, "\x1b[{}m{}\x1b[0m", c, name)?;
            } else {
                write!(out, "{}", name)?;
            }
        } else {
            write!(out, "{}", name)?;
        }

        Ok(())
    }

    #[cfg(unix)]
    fn print_long(
        entry: &DirEntry,
        out: &mut impl Write,
        human_readable: bool,
        use_color: bool,
    ) -> io::Result<()> {
        let metadata = fs::symlink_metadata(entry.path())?;
        let file_type = metadata.file_type();
        let mode = metadata.mode();

        let type_char = if file_type.is_dir() {
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

        let perms = Self::format_permissions(mode);
        let nlink = metadata.nlink();
        let uid = metadata.uid();
        let gid = metadata.gid();
        let size = metadata.size();
        let mtime = Self::format_time(metadata.modified()?);

        let name = entry.file_name().to_string_lossy().to_string();
        let displayed_name = if file_type.is_symlink() {
            match fs::read_link(entry.path()) {
                Ok(target) => format!("{} -> {}", name, target.display()),
                Err(_) => name.clone(),
            }
        } else {
            name.clone()
        };
        let name_display = if use_color {
            if let Some(color) = Self::get_color(&metadata, entry.path()) {
                format!("\x1b[{}m{}\x1b[0m", color, displayed_name)
            } else {
                displayed_name
            }
        } else {
            displayed_name
        };

        let size_display = if human_readable {
            Self::human_readable_size(size)
        } else {
            size.to_string()
        };

        writeln!(
            out,
            "{}{} {:>3} {:>5} {:>5} {:>8} {} {}",
            type_char, perms, nlink, uid, gid, size_display, mtime, name_display
        )?;

        Ok(())
    }

    #[cfg(not(unix))]
    fn print_long(
        entry: &DirEntry,
        out: &mut impl Write,
        human_readable: bool,
        use_color: bool,
    ) -> io::Result<()> {
        let metadata = fs::symlink_metadata(entry.path())?;
        let file_type = metadata.file_type();

        let type_char = if file_type.is_dir() {
            'd'
        } else if file_type.is_symlink() {
            'l'
        } else {
            '-'
        };

        let perms = Self::format_permissions_cross_platform(&metadata);
        let size = metadata.len();
        let mtime = Self::format_time(metadata.modified()?);

        let name = entry.file_name().to_string_lossy().to_string();
        let displayed_name = if file_type.is_symlink() {
            match fs::read_link(entry.path()) {
                Ok(target) => format!("{} -> {}", name, target.display()),
                Err(_) => name.clone(),
            }
        } else {
            name.clone()
        };
        let name_display = if use_color {
            if let Some(color) = Self::get_color(&metadata, entry.path()) {
                format!("\x1b[{}m{}\x1b[0m", color, displayed_name)
            } else {
                displayed_name
            }
        } else {
            displayed_name
        };

        let size_display = if human_readable {
            Self::human_readable_size(size)
        } else {
            size.to_string()
        };

        writeln!(
            out,
            "{}{} {:>3} {:>5} {:>5} {:>8} {} {}",
            type_char, perms, 1, "-", "-", size_display, mtime, name_display
        )?;

        Ok(())
    }

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

    #[cfg(not(unix))]
    fn format_permissions_cross_platform(metadata: &Metadata) -> String {
        let readonly = metadata.permissions().readonly();
        if readonly {
            "r--r--r--".to_string()
        } else {
            "rw-rw-rw-".to_string()
        }
    }

    fn format_time(time: SystemTime) -> String {
        let duration = time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs() as i64;

        let (_year, month, day, hour, min) = Self::unix_to_datetime(secs);

        format!(
            "{} {:>2} {:02}:{:02}",
            Self::month_name(month),
            day,
            hour,
            min
        )
    }

    fn unix_to_datetime(secs: i64) -> (i32, u32, u32, u32, u32) {
        const DAYS_PER_YEAR: i64 = 365;
        const DAYS_PER_LEAP: i64 = 366;

        let mut days = secs / 86400;
        let mut year = 1970;

        loop {
            let days_in_year = if Self::is_leap_year(year) {
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

        let leap = Self::is_leap_year(year);
        let days_in_months = if leap {
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

    fn human_readable_size(size: u64) -> String {
        human_size(size, false, true)
    }

    fn get_color(metadata: &Metadata, path: PathBuf) -> Option<&'static str> {
        let file_type = metadata.file_type();

        if file_type.is_dir() {
            return Some("1;34");
        }

        if file_type.is_symlink() {
            return Some("1;36");
        }

        if metadata.is_file() {
            #[cfg(unix)]
            {
                let mode = metadata.mode();
                if mode & 0o111 != 0 {
                    return Some("1;32");
                }
            }

            #[cfg(not(unix))]
            {
                if !metadata.permissions().readonly() {
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
}
