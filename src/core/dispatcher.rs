use crate::core::Applet;
use crate::applets::{EchoApplet, RelaxApplet};

pub struct Dispatcher;

impl Dispatcher {
    pub fn new() -> Self {
        Self
    }
    
    pub fn dispatch(&self, name: &str, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let has_help = args.iter().any(|a| a == "-h" || a == "--help");
        
        match name {
            "echo" => {
                let applet = EchoApplet;
                if has_help {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "relax" => {
                let applet = RelaxApplet;
                if has_help {
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
    
    pub fn list_applets(&self) {
        let applets: Vec<&dyn Applet> = vec![&EchoApplet, &RelaxApplet];
        for applet in applets {
            println!("{:<12} {}", applet.name(), applet.description());
        }
    }
}
