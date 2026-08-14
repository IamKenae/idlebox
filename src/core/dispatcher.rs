use crate::core::Applet;
use crate::applets::{
    CatApplet, ChmodApplet, CpApplet, DfApplet, DuApplet, EchoApplet, FreeApplet, GrepApplet,
    HeadApplet, KillApplet, LsApplet, MkdirApplet, MvApplet, PsApplet, RelaxApplet, RmApplet,
    TailApplet, TouchApplet, UptimeApplet,
};

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
            "chmod" => {
                let applet = ChmodApplet;
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
            "df" => {
                let applet = DfApplet;
                if has_help {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "du" => {
                let applet = DuApplet;
                if has_help {
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
            "free" => {
                let applet = FreeApplet;
                if has_help {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "grep" => {
                let applet = GrepApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "head" => {
                let applet = HeadApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "kill" => {
                let applet = KillApplet;
                if has_help {
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
            "ps" => {
                let applet = PsApplet;
                if has_help {
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
            "tail" => {
                let applet = TailApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "touch" => {
                let applet = TouchApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "uptime" => {
                let applet = UptimeApplet;
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
    
    pub fn applet_names(&self) -> Vec<&'static str> {
        vec!["cat", "chmod", "cp", "df", "du", "echo", "free", "grep", "head", "kill", "ls", "mkdir", "mv", "ps", "relax", "rm", "tail", "touch", "uptime"]
    }

    pub fn list_applets(&self) {
        let applets: Vec<&dyn Applet> = vec![
            &CatApplet, &ChmodApplet, &CpApplet, &DfApplet, &DuApplet, &EchoApplet,
            &FreeApplet, &GrepApplet, &HeadApplet, &KillApplet, &LsApplet, &MkdirApplet,
            &MvApplet, &PsApplet, &RelaxApplet, &RmApplet, &TailApplet, &TouchApplet,
            &UptimeApplet,
        ];
        for applet in applets {
            println!("{:<12} {}", applet.name(), applet.description());
        }
    }
}
