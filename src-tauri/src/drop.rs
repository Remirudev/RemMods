//! Native drag and drop onto the main window (PLAN §5; no HTML5 drag and drop). Paths are
//! classified and registered in Rust; the UI gets `install-drop` with ids and names only.

use std::path::PathBuf;

use remmods_ipc::{DropPhase, RejectedSource, SourceReject};
use tauri::{AppHandle, DragDropEvent, Manager};

use crate::events::{self, InstallDrop};
use crate::install_sources::{self, classify_all, register_all};
use crate::state::AppState;

/// Items handled per drop; the rest are refused as `tooMany`.
const MAX_DROPPED: usize = 32;

/// Runs on the main thread: enter and leave are emitted at once, a drop is classified on the
/// blocking pool. Positions and `Over` are ignored.
pub fn on_drag_drop(app: &AppHandle, event: &DragDropEvent) {
    match event {
        DragDropEvent::Enter { paths, .. } => emit(app, DropPhase::Enter, paths.len()),
        DragDropEvent::Leave => emit(app, DropPhase::Leave, 0),
        DragDropEvent::Drop { paths, .. } => {
            let app = app.clone();
            let paths = paths.clone();
            tauri::async_runtime::spawn_blocking(move || on_dropped(&app, paths));
        }
        _ => {}
    }
}

fn on_dropped(app: &AppHandle, mut paths: Vec<PathBuf>) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let count = count_of(paths.len());
    let extra = if paths.len() > MAX_DROPPED {
        paths.split_off(MAX_DROPPED)
    } else {
        Vec::new()
    };
    let classified = classify_all(&state.core, paths);
    let (sources, mut rejected) = match state.install_sources.lock() {
        Ok(mut registry) => register_all(&mut registry, classified),
        Err(_) => {
            log::error!("the install source registry is poisoned; ignoring the drop");
            return;
        }
    };
    rejected.extend(extra.iter().map(|path| RejectedSource {
        name: install_sources::display_name(path),
        reason: SourceReject::TooMany,
    }));
    log::info!(
        "drop: {} source(s) registered, {} refused",
        sources.len(),
        rejected.len()
    );
    events::emit(
        app,
        &InstallDrop {
            phase: DropPhase::Dropped,
            count,
            sources,
            rejected,
        },
    );
}

fn emit(app: &AppHandle, phase: DropPhase, count: usize) {
    events::emit(
        app,
        &InstallDrop {
            phase,
            count: count_of(count),
            sources: Vec::new(),
            rejected: Vec::new(),
        },
    );
}

fn count_of(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}
