use crate::core::Applet;
use crate::applets::{EchoApplet, RelaxApplet};

pub struct Dispatcher {
    applets: Vec<Box<dyn Applet>>,
}

impl Dispatcher {
    pub fn new() -> Self {
        let mut dispatcher = Self {
            applets: Vec::new(),
        };
        dispatcher.register_all();
        dispatcher
    }
    
    fn register_all(&mut self) {
        self.register(Box::new(EchoApplet));
        self.register(Box::new(RelaxApplet));
    }
    
    fn register(&mut self, applet: Box<dyn Applet>) {
        self.applets.push(applet);
    }
    
    pub fn dispatch(&self, name: &str, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        for applet in &self.applets {
            if applet.name() == name {
                return applet.run(args);
            }
        }
        Err(format!("idlebox: applet not found: {}", name).into())
    }
    
    pub fn list_applets(&self) {
        for applet in &self.applets {
            println!("{:<12} {}", applet.name(), applet.description());
        }
    }
}
