use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::task::JoinHandle;

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

const BRIGHT_CYAN: &str = "\x1b[96m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";
const CLEAR_LINE: &str = "\r\x1b[2K";

pub struct Spinner {
    cancel: Arc<AtomicBool>,
    label: Arc<Mutex<String>>,
    task: Option<JoinHandle<()>>,
}

impl Spinner {
    pub fn start(initial_label: impl Into<String>) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let label = Arc::new(Mutex::new(initial_label.into()));

        let cancel_clone = cancel.clone();
        let label_clone = label.clone();

        let task = tokio::spawn(async move {
            let mut i: usize = 0;
            let mut stderr = std::io::stderr();
            while !cancel_clone.load(Ordering::Relaxed) {
                let frame = FRAMES[i % FRAMES.len()];
                let text = label_clone.lock().map(|g| g.clone()).unwrap_or_default();
                let _ = write!(
                    stderr,
                    "{CLEAR_LINE}{BRIGHT_CYAN}{frame}{RESET} {DIM}{text}{RESET}"
                );
                let _ = stderr.flush();
                i = i.wrapping_add(1);
                tokio::time::sleep(Duration::from_millis(80)).await;
            }
            let _ = write!(stderr, "{CLEAR_LINE}");
            let _ = stderr.flush();
        });

        Self { cancel, label, task: Some(task) }
    }

    pub fn set_label(&self, s: impl Into<String>) {
        if let Ok(mut g) = self.label.lock() {
            *g = s.into();
        }
    }

    pub async fn stop(mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(t) = self.task.take() {
            let _ = t.await;
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(t) = self.task.take() {
            t.abort();
        }
    }
}
