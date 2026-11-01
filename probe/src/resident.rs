use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::monitor::{self, Shared};

pub fn run() {
    println!("\n# CONSOLE — monitor no terminal (Ctrl+C para sair)");
    let estado: Shared = Arc::new(Mutex::new(monitor::MonitorState::default()));
    monitor::iniciar(estado.clone());
    loop {
        thread::sleep(Duration::from_millis(400));
        let novas: Vec<String> = match estado.lock() {
            Ok(mut st) => st.log.drain(..).collect(),
            Err(_) => Vec::new(),
        };
        for linha in novas {
            println!("{linha}");
        }
    }
}
