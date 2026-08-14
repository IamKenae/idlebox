mod core;
mod applets;

use std::env;
use std::path::Path;
use std::process;

use crate::core::Dispatcher;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let argv0 = Path::new(&args[0])
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.trim_end_matches(".exe"))
        .unwrap_or("idlebox");
    
    let dispatcher = Dispatcher::new();
    
    let (applet_name, applet_args) = if argv0 == "idlebox" {
        if args.len() < 2 {
            print_usage(&dispatcher);
            process::exit(0);
        }
        (args[1].as_str(), &args[2..])
    } else {
        (argv0, &args[1..])
    };
    
    if applet_name == "list" {
        println!("Available applets:");
        dispatcher.list_applets();
        process::exit(0);
    }

    if applet_name == "--install" {
        let target = applet_args.first().map(|s| s.as_str());
        match crate::core::install::install(target) {
            Ok(code) => process::exit(code),
            Err(e) => {
                eprintln!("idlebox: install failed: {}", e);
                process::exit(1);
            }
        }
    }
    
    match dispatcher.dispatch(applet_name, applet_args) {
        Ok(exit_code) => process::exit(exit_code),
        Err(_) => process::exit(1),
    }
}

fn print_usage(dispatcher: &Dispatcher) {
    println!("IdleBox v0.1.0 - A modern BusyBox alternative");
    println!();
    println!("Usage:");
    println!("  idlebox <applet> [args...]    # Run an applet");
    println!("  idlebox list                  # List all applets");
    println!("  idlebox --install [PATH]      # Install symlinks for all applets");
    println!("  ./<applet> [args...]          # Run via symlink");
    println!();
    println!("Available applets:");
    dispatcher.list_applets();
}
