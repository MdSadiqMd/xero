use std::time::Duration;

use tauri::{ipc::Channel, AppHandle, Runtime};

use crate::{commands::RuntimeStreamItemDto, state::DesktopState};

const ATTACH_FRAME_POLL_INTERVAL: Duration = Duration::from_millis(200);
const ATTACH_RETRY_INTERVAL: Duration = Duration::from_millis(120);
const ATTACH_RETRY_ATTEMPTS: u32 = 4;
const TERMINAL_SNAPSHOT_RETRY_INTERVAL: Duration = Duration::from_millis(120);
const TERMINAL_SNAPSHOT_RETRY_ATTEMPTS: u32 = 6;
const PROTOCOL_LINE_LIMIT: usize = 16 * 1024;

mod attach;
mod controller;
mod items;
mod preflight;

pub use controller::{start_runtime_stream, RuntimeStreamController, RuntimeStreamRequest};

use controller::RuntimeStreamLease;
use items::{action_required_item, emit_item_if_requested, emit_terminal_item};
use preflight::{
    ensure_stream_identity, load_pending_action_required, load_streamable_runtime_run,
    load_terminal_runtime_snapshot,
};

#[derive(Debug)]
struct StreamFailure {
    error: crate::commands::CommandError,
    last_sequence: u64,
}

enum StreamExit {
    Cancelled,
    Failed(StreamFailure),
}

type StreamResult<T = u64> = Result<T, StreamExit>;

fn emit_runtime_stream<R: Runtime>(
    app: &AppHandle<R>,
    state: &DesktopState,
    request: &RuntimeStreamRequest,
    lease: &RuntimeStreamLease,
    channel: &Channel<RuntimeStreamItemDto>,
) -> StreamResult {
    ensure_stream_active(lease)?;
    ensure_stream_identity(app, state, request, 0)?;

    let runtime_run = load_streamable_runtime_run(request, 0)?;
    let pending_action_required = load_pending_action_required(request, 0)?;
    let mut attach_state =
        attach::attach_and_forward_supervisor_stream(request, lease, channel, &runtime_run)?;

    ensure_stream_active(lease)?;
    let terminal_snapshot =
        load_terminal_runtime_snapshot(state, request, attach_state.last_sequence)?;

    for approval in pending_action_required {
        if !attach_state
            .action_required_ids
            .insert(approval.action_id.clone())
        {
            continue;
        }

        attach_state.last_sequence = attach_state.last_sequence.saturating_add(1);
        emit_item_if_requested(
            channel,
            request,
            lease,
            action_required_item(request, attach_state.last_sequence, approval),
        )?;
    }

    emit_terminal_item(
        channel,
        request,
        lease,
        &terminal_snapshot,
        attach_state.last_sequence,
    )
}

fn ensure_stream_active(lease: &RuntimeStreamLease) -> StreamResult<()> {
    if lease.is_cancelled() {
        Err(StreamExit::Cancelled)
    } else {
        Ok(())
    }
}
