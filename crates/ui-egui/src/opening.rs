//! One bounded OPEN worker for the desktop UI. Programmatic OPEN stays synchronous.

use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};

use cadcraft_engine::{DocState, Session, Settings};

use crate::CadApp;

struct Job {
    path: String,
    settings: Box<Settings>,
    viewport: (f64, f64),
}

struct Finished {
    drawing: Result<DocState, String>,
    log: Vec<String>,
}

pub(super) struct Worker {
    send: SyncSender<Job>,
    receive: Receiver<Finished>,
    pending: usize,
}

impl Worker {
    fn start() -> Result<Self, String> {
        let (send, jobs) = mpsc::sync_channel::<Job>(8);
        // At most one decoded drawing waits in the channel while the UI is busy.
        let (done, receive) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("drawing-open".into())
            .spawn(move || {
                while let Ok(job) = jobs.recv() {
                    let mut session = Session::empty();
                    session.settings = *job.settings;
                    session.viewport_px = job.viewport;
                    // Reuse the guarded command, including format detection and xref resolution.
                    let drawing = session
                        .execute("open", &serde_json::json!({ "path": job.path }))
                        .map_err(|e| e.to_string())
                        .and_then(|_| session.docs.get(session.active).cloned().ok_or_else(|| "Open did not return a drawing".to_string()));
                    if done.send(Finished { drawing, log: session.log }).is_err() {
                        break;
                    }
                }
            })
            .map_err(|e| format!("Open: could not start the file reader: {e}"))?;
        Ok(Self { send, receive, pending: 0 })
    }
}

impl CadApp {
    /// Desktop hosts opt into background decoding. Embedded hosts and command/control calls
    /// retain synchronous OPEN semantics; the browser continues to open file bytes.
    pub fn enable_background_open(&mut self) {
        self.background_open = true;
    }

    pub(super) fn queue_open(&mut self, path: &str) {
        if self.opening.is_none() {
            match Worker::start() {
                Ok(worker) => self.opening = Some(worker),
                Err(e) => return self.open_failed(e),
            }
        }
        let job = Job { path: path.into(), settings: Box::new(self.session.settings.clone()), viewport: self.session.viewport_px };
        let sent = self.opening.as_mut().map(|w| w.send.try_send(job).map(|()| w.pending += 1));
        match sent {
            Some(Ok(())) => self.set_status(format!("Opening {path}…")),
            Some(Err(mpsc::TrySendError::Full(_))) => self.open_failed("Open: the file queue is full; try again after a drawing opens"),
            _ => {
                self.opening = None;
                self.open_failed("Open: the file reader stopped; try opening the drawing again");
            }
        }
    }

    pub(super) fn open_tick(&mut self, ctx: &egui::Context) {
        if self.quit_requested {
            self.opening = None;
            return;
        }
        loop {
            let received = match self.opening.as_mut() {
                Some(w) if w.pending > 0 => w.receive.try_recv(),
                _ => break,
            };
            match received {
                Ok(done) => {
                    if let Some(w) = &mut self.opening {
                        w.pending = w.pending.saturating_sub(1);
                    }
                    for message in done.log {
                        self.session.echo(message);
                    }
                    match done.drawing {
                        Ok(state) => {
                            let title = state.title.clone();
                            self.session.open_prepared(state);
                            self.opened(Ok(serde_json::Value::Null));
                            self.set_status(format!("Opened {title}"));
                        }
                        Err(e) => self.open_failed(e),
                    }
                    ctx.request_repaint();
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.opening = None;
                    self.open_failed("Open: the file reader stopped; try opening the drawing again");
                    break;
                }
            }
        }
        if self.opening.as_ref().is_some_and(|w| w.pending > 0) {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
}
