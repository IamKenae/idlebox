use crate::core::Applet;
use crate::applets::{CatApplet, CpApplet, EchoApplet, LsApplet, MkdirApplet, MvApplet, RelaxApplet, RmApplet};

pub struct Dispatcher;

impl Dispatcher {
    pub fn new() -> Self {
        Self
    }
    
    pub fn dispatch(&self, name: &str, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let has_help = args.iter().any(|a| a == "--help");
        let has_h_short = args.iter().any(|a| a == "-h");
        
        match name {
            "cat" => {
                let applet = CatApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "cp" => {
                let applet = CpApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "echo" => {
                let applet = EchoApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "ls" => {
                let applet = LsApplet;
                if has_help {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "mkdir" => {
                let applet = MkdirApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "mv" => {
                let applet = MvApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "relax" => {
                let applet = RelaxApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "rm" => {
                let applet = RmApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            _ => {
                eprintln!("idlebox: applet not found");
                Err("applet not found".into())
            }
        }
    }
    
    pub fn applet_names(&self) -> Vec<&'static str> {
        vec!["cat", "cp", "echo", "ls", "mkdir", "mv", "relax", "rm"]
    }

    pub fn list_applets(&self) {
        let applets: Vec<&dyn Applet> = vec![
            &CatApplet, &CpApplet, &EchoApplet, &LsApplet,
            &MkdirApplet, &MvApplet, &RelaxApplet, &RmApplet,
        ];
        for applet in applets {
            println!("{:<12} {}", applet.name(), applet.description());
        }
    }
}
