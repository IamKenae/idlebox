use crate::applets::{
    BracketApplet, CatApplet, ChgrpApplet, ChmodApplet, ChownApplet, CpApplet, CutApplet, DfApplet,
    DuApplet, EchoApplet, ExprApplet, FindApplet, FreeApplet, GrepApplet, HeadApplet, IdApplet,
    KillApplet, LnApplet, LsApplet, MkdirApplet, MvApplet, PsApplet, ReadlinkApplet, RelaxApplet,
    RmApplet, SortApplet, SuApplet, TailApplet, TestApplet, TouchApplet, TrApplet, UnameApplet,
    UniqApplet, UptimeApplet, WcApplet, WhoamiApplet,
};
use crate::core::Applet;

pub struct Dispatcher;

impl Dispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn dispatch(&self, name: &str, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut option_args = args.iter().take_while(|arg| arg.as_str() != "--");
        let has_help = option_args.clone().any(|arg| arg == "--help");
        let has_h_short = option_args.any(|arg| arg == "-h");

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
            "chgrp" => {
                let applet = ChgrpApplet;
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
            "chown" => {
                let applet = ChownApplet;
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
            "cut" => {
                let applet = CutApplet;
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
            "expr" => {
                let applet = ExprApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "find" => {
                let applet = FindApplet;
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
            "id" => {
                let applet = IdApplet;
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
            "ln" => {
                let applet = LnApplet;
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
            "ps" => {
                let applet = PsApplet;
                if has_help {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "readlink" => {
                let applet = ReadlinkApplet;
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
            "sort" => {
                let applet = SortApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "su" => {
                let applet = SuApplet;
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
            "test" => {
                let applet = TestApplet;
                if has_help {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "[" => {
                let applet = BracketApplet;
                if has_help {
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
            "tr" => {
                let applet = TrApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "uname" => {
                let applet = UnameApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "uniq" => {
                let applet = UniqApplet;
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
            "wc" => {
                let applet = WcApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            "whoami" => {
                let applet = WhoamiApplet;
                if has_help || has_h_short {
                    applet.help();
                    Ok(0)
                } else {
                    applet.run(args)
                }
            }
            _ => Err(format!("idlebox: '{}': applet not found", name).into()),
        }
    }

    #[cfg_attr(windows, allow(dead_code))]
    pub fn applet_names(&self) -> Vec<&'static str> {
        vec![
            "cat", "chgrp", "chmod", "chown", "cp", "cut", "df", "du", "echo", "expr", "find",
            "free", "grep", "head", "id", "kill", "ln", "ls", "mkdir", "mv", "ps", "readlink",
            "relax", "rm", "sort", "su", "tail", "test", "[", "touch", "tr", "uname", "uniq",
            "uptime", "wc", "whoami",
        ]
    }

    pub fn list_applets(&self) {
        let applets: Vec<&dyn Applet> = vec![
            &CatApplet,
            &ChgrpApplet,
            &ChmodApplet,
            &ChownApplet,
            &CpApplet,
            &CutApplet,
            &DfApplet,
            &DuApplet,
            &EchoApplet,
            &ExprApplet,
            &FindApplet,
            &FreeApplet,
            &GrepApplet,
            &HeadApplet,
            &IdApplet,
            &KillApplet,
            &LnApplet,
            &LsApplet,
            &MkdirApplet,
            &MvApplet,
            &PsApplet,
            &ReadlinkApplet,
            &RelaxApplet,
            &RmApplet,
            &SortApplet,
            &SuApplet,
            &TailApplet,
            &TestApplet,
            &BracketApplet,
            &TouchApplet,
            &TrApplet,
            &UnameApplet,
            &UniqApplet,
            &UptimeApplet,
            &WcApplet,
            &WhoamiApplet,
        ];
        for applet in applets {
            println!("{:<12} {}", applet.name(), applet.description());
        }
    }
}
