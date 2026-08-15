mod applets;
mod core;

use std::env;
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
        let options_ended = applet_args.first().is_some_and(|arg| arg == "--");
        let install_args = if options_ended {
            &applet_args[1..]
        } else {
            applet_args
        };

        if install_args.len() == 1 && matches!(install_args[0].as_str(), "-h" | "--help") {
            print_install_usage();
            process::exit(0);
        }
        if !options_ended
            && install_args
                .first()
                .is_some_and(|argument| argument.starts_with('-'))
        {
            eprintln!("idlebox: --install: unknown option '{}'", install_args[0]);
            eprintln!("Use '--' before PATH when the path starts with '-'.");
            process::exit(1);
        }
        if install_args.len() > 1 {
            eprintln!(
                "idlebox: --install: unexpected argument '{}'",
                install_args[1]
            );
            process::exit(1);
        }

        let target = install_args.first().map(String::as_str);
        match crate::core::install::install(target) {
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
            eprintln!("{}", error);
            process::exit(1);
        }
    }
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
    println!(
        "IdleBox v{} - A lightweight multi-call toolbox",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("Usage:");
    println!("  idlebox <applet> [args...]    # Run an applet");
    println!("  idlebox help [APPLET]         # Show general or applet help");
    println!("  idlebox list                  # List all applets");
    println!("  idlebox --install [PATH]      # Install launchers for all applets");
    println!("  idlebox --version             # Print the IdleBox version");
    println!("  ./<applet> [args...]          # Run via an installed launcher");
    println!();
    println!("Available applets:");
    dispatcher.list_applets();
}

fn print_install_usage() {
    println!("Usage: idlebox --install [PATH]");
    println!();
    println!("Install launchers for every registered applet.");
    println!("Use '--' before PATH when the path starts with '-'.");
}

fn print_list_usage() {
    println!("Usage: idlebox list");
    println!();
    println!("List every registered applet and its description.");
}

#[cfg(test)]
mod tests {
    use super::strip_exe_suffix;

    #[test]
    fn strips_exe_suffix_case_insensitively() {
        assert_eq!(strip_exe_suffix("idlebox.exe"), "idlebox");
        assert_eq!(strip_exe_suffix("idlebox.EXE"), "idlebox");
        assert_eq!(strip_exe_suffix("idlebox"), "idlebox");
        assert_eq!(strip_exe_suffix("盒.exe"), "盒");
    }
}
