use p2_bridge_core::{auth, gateway::Authorization};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
};

pub enum ApprovalEvent {
    Challenge(String),
    Complete(Result<Authorization, &'static str>),
}
pub struct PendingApproval {
    cancelled: Arc<AtomicBool>,
    pub events: Receiver<ApprovalEvent>,
}
impl PendingApproval {
    pub fn start() -> Self {
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = cancelled.clone();
        let (tx, events) = mpsc::channel();
        thread::spawn(move || {
            let result = perform(&flag, &tx);
            // A cancelled or superseded worker never delivers an approval.
            let result = if flag.load(Ordering::Acquire) {
                Err("Approval cancelled.")
            } else {
                result
            };
            let _ = tx.send(ApprovalEvent::Complete(result));
        });
        Self { cancelled, events }
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
}
impl Drop for PendingApproval {
    fn drop(&mut self) {
        self.cancel();
    }
}
fn perform(flag: &AtomicBool, tx: &Sender<ApprovalEvent>) -> Result<Authorization, &'static str> {
    let endpoints = auth::directory().map_err(|_| {
        "Could not reach the Steam server directory. Check your connection and retry."
    })?;
    if flag.load(Ordering::Acquire) {
        return Err("Approval cancelled.");
    }
    let cm = *endpoints
        .first()
        .ok_or("Steam returned no usable server.")?;
    auth::authorize_interactive(cm, &|| flag.load(Ordering::Acquire), |url| {
        tx.send(ApprovalEvent::Challenge(url.to_owned()))
            .map_err(|_| io::Error::new(io::ErrorKind::Interrupted, "window_closed"))
    })
    .map_err(|e| {
        if e.kind() == io::ErrorKind::Interrupted {
            "Approval cancelled."
        } else {
            "Steam approval failed or expired. Request a new QR code."
        }
    })
}
