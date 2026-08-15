mod applets;
mod core;

use std::env;
use std::error::Error;
use std::io;
use std::path::Path;
use std::process;

use crate::core::Dispatcher;

fn main() {
    let args: Vec<String> = env::args().collect();

    let argv0 = args
        .first()
        .map(String::as_str)
        .and_then(|arg| Path::new(arg).file_name())
        .and_then(|name| name.to_str())
        .map(strip_exe_suffix)
        .unwrap_or("idlebox");

    let dispatcher = Dispatcher::new();
    let invoked_as_idlebox = argv0.eq_ignore_ascii_case("idlebox");

    let (applet_name, applet_args) = if invoked_as_idlebox {
        if args.len() < 2 {
            print_usage(&dispatcher);
            process::exit(0);
        }
        (args[1].as_str(), &args[2..])
    } else {
        (argv0, &args[1..])
    };

    if invoked_as_idlebox {
        match applet_name {
            "-h" | "--help" => {
                print_usage(&dispatcher);
                process::exit(0);
            }
            "-V" | "--version" => {
                println!("idlebox {}", env!("CARGO_PKG_VERSION"));
                process::exit(0);
            }
            "help" => {
                if applet_args.is_empty() {
                    print_usage(&dispatcher);
                    process::exit(0);
                }
                if applet_args.len() > 1 {
                    eprintln!("idlebox: help: expected at most one applet name");
                    process::exit(1);
                }

                match applet_args[0].as_str() {
                    "--install" => print_install_usage(),
                    "list" | "--list" => print_list_usage(),
                    name => {
                        if let Err(error) = dispatcher.help(name) {
                            eprintln!("{}", error);
                            process::exit(1);
                        }
                    }
                }
                process::exit(0);
            }
            _ => {}
        }
    }

    if applet_name == "list" || (invoked_as_idlebox && applet_name == "--list") {
        if !applet_args.is_empty() {
            eprintln!("idlebox: list: unexpected argument '{}'", applet_args[0]);
            process::exit(1);
        }
        println!("Available applets:");
        dispatcher.list_applets();
        process::exit(0);
    }

    if invoked_as_idlebox && applet_name == "--install" {
        let mut target = None;
        let mut force = false;
        let mut dry_run = false;
        let mut options_ended = false;

        for argument in applet_args {
            if !options_ended {
                match argument.as_str() {
                    "--" => {
                        options_ended = true;
                        continue;
                    }
                    "-h" | "--help" => {
                        print_install_usage();
                        process::exit(0);
                    }
                    "--force" => {
                        force = true;
                        continue;
                    }
                    "--dry-run" => {
                        dry_run = true;
                        continue;
                    }
                    option if option.starts_with('-') => {
                        eprintln!("idlebox: --install: unknown option '{}'", option);
                        eprintln!("Use '--' before PATH when the path starts with '-'.");
                        process::exit(1);
                    }
                    _ => {}
                }
            }

            if target.is_some() {
                eprintln!("idlebox: --install: unexpected argument '{}'", argument);
                process::exit(1);
            }
            target = Some(argument.as_str());
        }

        let options = crate::core::install::InstallOptions {
            target,
            force,
            dry_run,
        };
        match crate::core::install::install(options) {
            Ok(code) => process::exit(code),
            Err(error) => {
                eprintln!("idlebox: install failed: {}", error);
                process::exit(1);
            }
        }
    }

    match dispatcher.dispatch(applet_name, applet_args) {
        Ok(exit_code) => process::exit(exit_code),
        Err(error) => {
            if is_broken_pipe(error.as_ref()) {
                process::exit(0);
            }
            eprintln!("{}", error);
            process::exit(1);
        }
    }
}

fn is_broken_pipe(error: &(dyn Error + 'static)) -> bool {
    let mut current = Some(error);
    while let Some(cause) = current {
        if cause
            .downcast_ref::<io::Error>()
            .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe)
        {
            return true;
        }
        current = cause.source();
    }
    false
}

fn strip_exe_suffix(name: &str) -> &str {
    let suffix_start = name.len().saturating_sub(4);
    if name.as_bytes()[suffix_start..].eq_ignore_ascii_case(b".exe") {
        &name[..suffix_start]
    } else {
        name
    }
}

fn print_usage(dispatcher: &Dispatcher) {
    println!("{}", crate::core::banner());
    println!();
    println!("Usage:");
    println!("  idlebox <applet> [args...]    # Run an applet");
    println!("  idlebox help [APPLET]         # Show general or applet help");
    println!("  idlebox list                  # List all applets");
    println!("  idlebox --install [OPTIONS] [PATH] # Install launchers for all applets");
    println!("  idlebox --version             # Print the IdleBox version");
    println!("  ./<applet> [args...]          # Run via an installed launcher");
    println!();
    println!("Available applets:");
    dispatcher.list_applets();
}

fn print_install_usage() {
    println!("Usage: idlebox --install [OPTIONS] [PATH]");
    println!();
    println!("Install launchers for every registered applet.");
    println!();
    println!("Options:");
    println!("  --force       Replace existing files and links");
    println!("  --dry-run     Preview changes without writing anything");
    println!("  -h, --help    Print this help");
    println!();
    println!("Existing directories are never replaced.");
    println!("Use '--' before PATH when the path starts with '-'.");
}

fn print_list_usage() {
    println!("Usage: idlebox list");
    println!();
    println!("List every registered applet and its description.");
}

#[cfg(test)]
mod tests {
    use super::{is_broken_pipe, strip_exe_suffix};
    use std::io;

    #[test]
    fn strips_exe_suffix_case_insensitively() {
        assert_eq!(strip_exe_suffix("idlebox.exe"), "idlebox");
        assert_eq!(strip_exe_suffix("idlebox.EXE"), "idlebox");
        assert_eq!(strip_exe_suffix("idlebox"), "idlebox");
        assert_eq!(strip_exe_suffix("盒.exe"), "盒");
    }

    #[test]
    fn recognizes_broken_pipe_errors() {
        let error = io::Error::new(io::ErrorKind::BrokenPipe, "closed pipe");
        assert!(is_broken_pipe(&error));
    }
}
