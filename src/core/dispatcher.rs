use crate::applets::{
    BracketApplet, CatApplet, ChgrpApplet, ChmodApplet, ChownApplet, CpApplet, CutApplet, DfApplet,
    DuApplet, EchoApplet, ExprApplet, FindApplet, FreeApplet, GrepApplet, HeadApplet, IdApplet,
    KillApplet, LnApplet, LsApplet, MkdirApplet, MvApplet, PsApplet, ReadlinkApplet, RelaxApplet,
    RmApplet, SortApplet, SuApplet, TailApplet, TestApplet, TouchApplet, TrApplet, UnameApplet,
    UniqApplet, UptimeApplet, WcApplet, WhoamiApplet,
};
use crate::core::Applet;

struct AppletEntry {
    applet: &'static dyn Applet,
    short_help: bool,
}

impl AppletEntry {
    const fn new(applet: &'static dyn Applet, short_help: bool) -> Self {
        Self { applet, short_help }
    }
}

// Keep registration, listing, installation, and help policy in one place. Applets
// whose `-h` flag has command-specific meaning only accept the long help flag.
static APPLETS: &[AppletEntry] = &[
    AppletEntry::new(&CatApplet, true),
    AppletEntry::new(&ChgrpApplet, true),
    AppletEntry::new(&ChmodApplet, true),
    AppletEntry::new(&ChownApplet, true),
    AppletEntry::new(&CpApplet, true),
    AppletEntry::new(&CutApplet, true),
    AppletEntry::new(&DfApplet, false),
    AppletEntry::new(&DuApplet, false),
    AppletEntry::new(&EchoApplet, true),
    AppletEntry::new(&ExprApplet, true),
    AppletEntry::new(&FindApplet, true),
    AppletEntry::new(&FreeApplet, false),
    AppletEntry::new(&GrepApplet, true),
    AppletEntry::new(&HeadApplet, true),
    AppletEntry::new(&IdApplet, true),
    AppletEntry::new(&KillApplet, false),
    AppletEntry::new(&LnApplet, true),
    AppletEntry::new(&LsApplet, false),
    AppletEntry::new(&MkdirApplet, true),
    AppletEntry::new(&MvApplet, true),
    AppletEntry::new(&PsApplet, false),
    AppletEntry::new(&ReadlinkApplet, false),
    AppletEntry::new(&RelaxApplet, true),
    AppletEntry::new(&RmApplet, true),
    AppletEntry::new(&SortApplet, true),
    AppletEntry::new(&SuApplet, true),
    AppletEntry::new(&TailApplet, true),
    AppletEntry::new(&TestApplet, false),
    AppletEntry::new(&BracketApplet, false),
    AppletEntry::new(&TouchApplet, true),
    AppletEntry::new(&TrApplet, true),
    AppletEntry::new(&UnameApplet, true),
    AppletEntry::new(&UniqApplet, true),
    AppletEntry::new(&UptimeApplet, false),
    AppletEntry::new(&WcApplet, true),
    AppletEntry::new(&WhoamiApplet, true),
];

pub struct Dispatcher;

impl Dispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn dispatch(&self, name: &str, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let entry = self.find(name).ok_or_else(|| Self::not_found(name))?;

        if Self::help_requested(entry, args) {
            entry.applet.help();
            Ok(0)
        } else {
            entry.applet.run(args)
        }
    }

    pub fn help(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let entry = self.find(name).ok_or_else(|| Self::not_found(name))?;
        entry.applet.help();
        Ok(())
    }

    pub fn applet_names(&self) -> impl ExactSizeIterator<Item = &'static str> {
        APPLETS.iter().map(|entry| entry.applet.name())
    }

    pub fn list_applets(&self) {
        for entry in APPLETS {
            println!("{:<12} {}", entry.applet.name(), entry.applet.description());
        }
    }

    fn find(&self, name: &str) -> Option<&'static AppletEntry> {
        APPLETS.iter().find(|entry| entry.applet.name() == name)
    }

    fn help_requested(entry: &AppletEntry, args: &[String]) -> bool {
        args.iter()
            .take_while(|arg| arg.as_str() != "--")
            .any(|arg| arg == "--help" || (entry.short_help && arg == "-h"))
    }

    fn not_found(name: &str) -> Box<dyn std::error::Error> {
        format!(
            "idlebox: '{}': applet not found\nTry 'idlebox list' to see available applets.",
            name
        )
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::APPLETS;

    #[test]
    fn applet_registry_has_unique_names() {
        for (index, entry) in APPLETS.iter().enumerate() {
            let name = entry.applet.name();
            assert!(
                APPLETS[..index]
                    .iter()
                    .all(|registered| registered.applet.name() != name),
                "duplicate applet registration: {name}"
            );
        }
    }
}
