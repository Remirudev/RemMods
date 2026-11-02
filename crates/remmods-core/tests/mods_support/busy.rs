//! Folder-busy sinks for the WG-2 tests (P3b §5.7): a recorder, optionally cancelling through a
//! `BusyCanceller` (no Arc around Core or OpGate, no cycle).

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use remmods_core::platform::RetryPolicy;
use remmods_core::safe_fs::{BusyEvent, BusyPhase, BusyWaits};

#[derive(Clone, Default)]
pub struct Recorder {
    events: Arc<Mutex<Vec<BusyEvent>>>,
    cancels: Arc<Mutex<Vec<bool>>>,
}

impl Recorder {
    pub fn events(&self) -> Vec<BusyEvent> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn phases(&self) -> Vec<BusyPhase> {
        self.events().iter().map(|event| event.phase).collect()
    }

    /// What every `cancel` the sink sent returned.
    pub fn cancels(&self) -> Vec<bool> {
        self.cancels
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Records every event.
pub fn record(busy: &BusyWaits) -> Recorder {
    install(busy, None)
}

/// Records every event and cancels its wait on each event of `phase`.
pub fn cancel_on(busy: &BusyWaits, phase: BusyPhase) -> Recorder {
    install(busy, Some(phase))
}

fn install(busy: &BusyWaits, cancel_phase: Option<BusyPhase>) -> Recorder {
    let recorder = Recorder::default();
    let sink_recorder = recorder.clone();
    let canceller = busy.canceller();
    busy.set_sink(Arc::new(move |event: &BusyEvent| {
        sink_recorder
            .events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event.clone());
        if cancel_phase == Some(event.phase) {
            let accepted = canceller.cancel(event.wait_id);
            sink_recorder
                .cancels
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(accepted);
        }
    }));
    recorder
}

/// A folder policy override: `attempts` tries spread over `total_ms`.
pub fn policy(attempts: u32, total_ms: u64) -> RetryPolicy {
    RetryPolicy {
        attempts,
        total: Duration::from_millis(total_ms),
    }
}
