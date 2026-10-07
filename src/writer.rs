//! Runs the board's bd writes (claim, close, move, priority, note, comment,
//! create, edit) on one background worker, so the board never freezes while
//! bd works. One worker keeps the writes in the order they were made.

use crate::form::CreateForm;
use anyhow::Result;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

type Job = Box<dyn FnOnce() -> Result<String> + Send>;

/// A finished write.
pub struct Done {
    /// The success message, or bd's error.
    pub result: Result<String, String>,
    /// The form to reopen when a create or edit failed, so nothing typed is lost.
    pub form: Option<CreateForm>,
}

pub struct Writer {
    jobs: Sender<(Job, Option<CreateForm>)>,
    done: Receiver<Done>,
}

impl Writer {
    pub fn start() -> Self {
        let (jobs, inbox) = mpsc::channel::<(Job, Option<CreateForm>)>();
        let (outbox, done) = mpsc::channel();
        thread::spawn(move || {
            for (job, form) in inbox {
                let result = job().map_err(|e| e.to_string());
                let form = if result.is_err() { form } else { None };
                if outbox.send(Done { result, form }).is_err() {
                    return;
                }
            }
        });
        Writer { jobs, done }
    }

    /// Queue a write. `form` is handed back if it fails.
    pub fn submit(
        &self,
        job: impl FnOnce() -> Result<String> + Send + 'static,
        form: Option<CreateForm>,
    ) {
        let _ = self.jobs.send((Box::new(job), form));
    }

    /// Finished writes, without blocking.
    pub fn poll(&self) -> Vec<Done> {
        self.done.try_iter().collect()
    }
}
