use std::path::Path;
use std::time::Instant;
use process_list::for_each_process;

pub struct TarkovDetector {
    last_time: Instant,
    is_running: bool,
}

impl TarkovDetector {

    pub fn new() -> Self {
        return Self {
            last_time: Instant::now(),
            is_running: false
        }
    }

    pub fn tarkov_running(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_time).as_secs() > 10 {
            self.last_time = now;
            let mut is_running = false;
            for_each_process(|_id: u32, name: &Path| {
                if name.display().to_string().contains("EscapeFromTarkov.exe") {
                    is_running = true;
                }
            }).unwrap();
            self.is_running = is_running;
        }
        return self.is_running;
    }
}