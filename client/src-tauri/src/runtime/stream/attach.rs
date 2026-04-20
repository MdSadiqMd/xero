use std::{
    collections::HashSet,
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpStream},
};

use tauri::ipc::Channel;

use crate::{
    commands::{
        runtime_support::DEFAULT_RUNTIME_RUN_CONTROL_TIMEOUT, CommandError, RuntimeStreamItemDto,
    },
    db::project_store::RuntimeRunSnapshotRecord,
    runtime::protocol::{
        SupervisorControlRequest, SupervisorControlResponse, SUPERVISOR_PROTOCOL_VERSION,
    },
};

use super::{
    controller::{RuntimeStreamLease, RuntimeStreamRequest},
    ensure_stream_active,
    items::{emit_item_if_requested, map_supervisor_event_to_stream_item},
    StreamExit, StreamFailure, StreamResult, ATTACH_FRAME_POLL_INTERVAL, ATTACH_RETRY_ATTEMPTS,
    ATTACH_RETRY_INTERVAL, PROTOCOL_LINE_LIMIT,
};

#[derive(Debug)]
struct AttachAck {
    replayed_count: u32,
}

#[derive(Debug, Default)]
pub(super) struct AttachForwardState {
    pub(super) last_sequence: u64,
    pub(super) action_required_ids: HashSet<String>,
}

enum ReadSupervisorResponseError {
    Timeout,
    Io(std::io::Error),
    Decode(String),
}

pub(super) fn attach_and_forward_supervisor_stream(
    request: &RuntimeStreamRequest,
    lease: &RuntimeStreamLease,
    channel: &Channel<RuntimeStreamItemDto>,
    runtime_run: &RuntimeRunSnapshotRecord,
) -> StreamResult<AttachForwardState> {
    let (mut reader, attach_ack) =
        open_attach_reader_with_ack(request, lease, &runtime_run.run.transport.endpoint)?;
    let mut attach_state = AttachForwardState::default();

    for _ in 0..attach_ack.replayed_count {
        ensure_stream_active(lease)?;
        attach_state = read_and_forward_event(&mut reader, request, lease, channel, attach_state)?;
    }

    loop {
        ensure_stream_active(lease)?;
        match read_supervisor_response(&mut reader) {
            Ok(Some(response)) => {
                attach_state =
                    forward_supervisor_response(response, request, lease, channel, attach_state)?;
            }
            Ok(None) => return Ok(attach_state),
            Err(ReadSupervisorResponseError::Timeout) => continue,
            Err(ReadSupervisorResponseError::Io(error)) => {
                return Err(StreamExit::Failed(StreamFailure {
                    error: CommandError::retryable(
                        "runtime_stream_attach_io_failed",
                        format!(
                            "Cadence lost the detached supervisor attach stream while bridging live runtime events: {error}"
                        ),
                    ),
                    last_sequence: attach_state.last_sequence,
                }));
            }
            Err(ReadSupervisorResponseError::Decode(error)) => {
                return Err(StreamExit::Failed(StreamFailure {
                    error: CommandError::system_fault(
                        "runtime_stream_contract_invalid",
                        format!(
                            "Cadence could not decode a detached supervisor attach frame while bridging the live runtime stream: {error}"
                        ),
                    ),
                    last_sequence: attach_state.last_sequence,
                }));
            }
        }
    }
}

fn open_attach_reader_with_ack(
    request: &RuntimeStreamRequest,
    lease: &RuntimeStreamLease,
    endpoint: &str,
) -> StreamResult<(BufReader<TcpStream>, AttachAck)> {
    let mut last_failure: Option<StreamFailure> = None;

    for attempt in 0..ATTACH_RETRY_ATTEMPTS {
        ensure_stream_active(lease)?;

        let mut reader = match open_attach_reader(request, endpoint, 0) {
            Ok(reader) => reader,
            Err(StreamExit::Failed(failure)) => {
                last_failure = Some(failure);
                if attempt + 1 < ATTACH_RETRY_ATTEMPTS {
                    std::thread::sleep(ATTACH_RETRY_INTERVAL);
                    continue;
                }

                return Err(StreamExit::Failed(
                    last_failure.expect("attach failure captured"),
                ));
            }
            Err(other) => return Err(other),
        };

        match read_attach_ack(&mut reader, request, 0) {
            Ok(attach_ack) => return Ok((reader, attach_ack)),
            Err(StreamExit::Failed(failure)) => {
                last_failure = Some(failure);
                if attempt + 1 < ATTACH_RETRY_ATTEMPTS {
                    std::thread::sleep(ATTACH_RETRY_INTERVAL);
                    continue;
                }

                return Err(StreamExit::Failed(
                    last_failure.expect("attach failure captured"),
                ));
            }
            Err(other) => return Err(other),
        }
    }

    Err(StreamExit::Failed(last_failure.unwrap_or(StreamFailure {
        error: CommandError::retryable(
            "runtime_stream_attach_connect_failed",
            format!(
                "Cadence could not connect the live runtime stream to detached run `{}`.",
                request.run_id
            ),
        ),
        last_sequence: 0,
    })))
}

fn open_attach_reader(
    request: &RuntimeStreamRequest,
    endpoint: &str,
    last_sequence: u64,
) -> StreamResult<BufReader<TcpStream>> {
    let address = endpoint.parse::<SocketAddr>().map_err(|_| {
        StreamExit::Failed(StreamFailure {
            error: CommandError::retryable(
                "runtime_supervisor_endpoint_invalid",
                "Cadence could not parse the detached supervisor control endpoint for the live runtime stream.",
            ),
            last_sequence,
        })
    })?;

    let mut stream = TcpStream::connect_timeout(&address, DEFAULT_RUNTIME_RUN_CONTROL_TIMEOUT)
        .map_err(|_| {
            StreamExit::Failed(StreamFailure {
                error: CommandError::retryable(
                    "runtime_stream_attach_connect_failed",
                    format!(
                        "Cadence could not connect the live runtime stream to detached run `{}`.",
                        request.run_id
                    ),
                ),
                last_sequence,
            })
        })?;

    stream
        .set_write_timeout(Some(DEFAULT_RUNTIME_RUN_CONTROL_TIMEOUT))
        .map_err(|_| {
            StreamExit::Failed(StreamFailure {
                error: CommandError::retryable(
                    "runtime_stream_attach_timeout_config_failed",
                    "Cadence could not configure the live runtime stream attach write timeout.",
                ),
                last_sequence,
            })
        })?;
    stream
        .set_read_timeout(Some(DEFAULT_RUNTIME_RUN_CONTROL_TIMEOUT))
        .map_err(|_| {
            StreamExit::Failed(StreamFailure {
                error: CommandError::retryable(
                    "runtime_stream_attach_timeout_config_failed",
                    "Cadence could not configure the live runtime stream attach read timeout.",
                ),
                last_sequence,
            })
        })?;

    write_json_line(
        &mut stream,
        &SupervisorControlRequest::attach(&request.project_id, &request.run_id, None),
    )
    .map_err(|error| {
        StreamExit::Failed(StreamFailure {
            error: CommandError::retryable(
                "runtime_stream_attach_write_failed",
                format!(
                    "Cadence could not send the detached supervisor attach request for run `{}`: {error}",
                    request.run_id
                ),
            ),
            last_sequence,
        })
    })?;

    stream
        .set_read_timeout(Some(ATTACH_FRAME_POLL_INTERVAL))
        .map_err(|_| {
            StreamExit::Failed(StreamFailure {
                error: CommandError::retryable(
                    "runtime_stream_attach_timeout_config_failed",
                    "Cadence could not switch the live runtime stream attach socket into polling mode.",
                ),
                last_sequence,
            })
        })?;

    Ok(BufReader::new(stream))
}

fn read_attach_ack(
    reader: &mut BufReader<TcpStream>,
    request: &RuntimeStreamRequest,
    last_sequence: u64,
) -> StreamResult<AttachAck> {
    match read_supervisor_response(reader) {
        Ok(Some(SupervisorControlResponse::Attached {
            protocol_version,
            project_id,
            run_id,
            replayed_count,
            ..
        })) => {
            if protocol_version != SUPERVISOR_PROTOCOL_VERSION {
                return Err(StreamExit::Failed(StreamFailure {
                    error: CommandError::retryable(
                        "runtime_stream_contract_invalid",
                        "Cadence rejected the detached supervisor attach acknowledgement because its protocol version was unsupported.",
                    ),
                    last_sequence,
                }));
            }

            if project_id != request.project_id || run_id != request.run_id {
                return Err(StreamExit::Failed(StreamFailure {
                    error: CommandError::retryable(
                        "runtime_stream_run_replaced",
                        "Cadence rejected the detached supervisor attach acknowledgement because it no longer matched the active project or run.",
                    ),
                    last_sequence,
                }));
            }

            Ok(AttachAck { replayed_count })
        }
        Ok(Some(SupervisorControlResponse::Error {
            code,
            message,
            retryable,
            ..
        })) => Err(StreamExit::Failed(StreamFailure {
            error: if retryable {
                CommandError::retryable(code, message)
            } else {
                CommandError::user_fixable(code, message)
            },
            last_sequence,
        })),
        Ok(Some(other)) => Err(StreamExit::Failed(StreamFailure {
            error: CommandError::system_fault(
                "runtime_stream_contract_invalid",
                format!(
                    "Cadence expected a detached supervisor attach acknowledgement but received `{other:?}` instead."
                ),
            ),
            last_sequence,
        })),
        Ok(None) => Err(StreamExit::Failed(StreamFailure {
            error: CommandError::retryable(
                "runtime_stream_attach_closed",
                "Cadence lost the detached supervisor attach stream before the acknowledgement arrived.",
            ),
            last_sequence,
        })),
        Err(ReadSupervisorResponseError::Timeout) => Err(StreamExit::Failed(StreamFailure {
            error: CommandError::retryable(
                "runtime_stream_attach_timeout",
                format!(
                    "Cadence timed out while waiting for detached supervisor run `{}` to acknowledge the live stream attach.",
                    request.run_id
                ),
            ),
            last_sequence,
        })),
        Err(ReadSupervisorResponseError::Io(error)) => Err(StreamExit::Failed(StreamFailure {
            error: CommandError::retryable(
                "runtime_stream_attach_io_failed",
                format!(
                    "Cadence lost the detached supervisor attach stream before the acknowledgement completed: {error}"
                ),
            ),
            last_sequence,
        })),
        Err(ReadSupervisorResponseError::Decode(error)) => Err(StreamExit::Failed(StreamFailure {
            error: CommandError::system_fault(
                "runtime_stream_contract_invalid",
                format!(
                    "Cadence could not decode the detached supervisor attach acknowledgement: {error}"
                ),
            ),
            last_sequence,
        })),
    }
}

fn read_and_forward_event(
    reader: &mut BufReader<TcpStream>,
    request: &RuntimeStreamRequest,
    lease: &RuntimeStreamLease,
    channel: &Channel<RuntimeStreamItemDto>,
    attach_state: AttachForwardState,
) -> StreamResult<AttachForwardState> {
    match read_supervisor_response(reader) {
        Ok(Some(response)) => {
            forward_supervisor_response(response, request, lease, channel, attach_state)
        }
        Ok(None) => Err(StreamExit::Failed(StreamFailure {
            error: CommandError::retryable(
                "runtime_stream_attach_closed",
                "Cadence lost the detached supervisor attach stream while replaying buffered runtime events.",
            ),
            last_sequence: attach_state.last_sequence,
        })),
        Err(ReadSupervisorResponseError::Timeout) => Err(StreamExit::Failed(StreamFailure {
            error: CommandError::retryable(
                "runtime_stream_attach_timeout",
                "Cadence timed out while replaying buffered runtime events from the detached supervisor.",
            ),
            last_sequence: attach_state.last_sequence,
        })),
        Err(ReadSupervisorResponseError::Io(error)) => Err(StreamExit::Failed(StreamFailure {
            error: CommandError::retryable(
                "runtime_stream_attach_io_failed",
                format!(
                    "Cadence lost the detached supervisor attach stream while replaying buffered runtime events: {error}"
                ),
            ),
            last_sequence: attach_state.last_sequence,
        })),
        Err(ReadSupervisorResponseError::Decode(error)) => Err(StreamExit::Failed(StreamFailure {
            error: CommandError::system_fault(
                "runtime_stream_contract_invalid",
                format!(
                    "Cadence could not decode a replayed detached supervisor event frame: {error}"
                ),
            ),
            last_sequence: attach_state.last_sequence,
        })),
    }
}

fn forward_supervisor_response(
    response: SupervisorControlResponse,
    request: &RuntimeStreamRequest,
    lease: &RuntimeStreamLease,
    channel: &Channel<RuntimeStreamItemDto>,
    mut attach_state: AttachForwardState,
) -> StreamResult<AttachForwardState> {
    match response {
        SupervisorControlResponse::Event {
            protocol_version,
            project_id,
            run_id,
            sequence,
            created_at,
            item,
            ..
        } => {
            if protocol_version != SUPERVISOR_PROTOCOL_VERSION {
                return Err(StreamExit::Failed(StreamFailure {
                    error: CommandError::system_fault(
                        "runtime_stream_contract_invalid",
                        "Cadence rejected a detached supervisor event frame because its protocol version was unsupported.",
                    ),
                    last_sequence: attach_state.last_sequence,
                }));
            }

            if project_id != request.project_id || run_id != request.run_id {
                return Err(StreamExit::Failed(StreamFailure {
                    error: CommandError::retryable(
                        "runtime_stream_run_replaced",
                        "Cadence rejected a detached supervisor event frame because it no longer matched the active project or run.",
                    ),
                    last_sequence: attach_state.last_sequence,
                }));
            }

            if sequence == 0 || sequence <= attach_state.last_sequence {
                return Err(StreamExit::Failed(StreamFailure {
                    error: CommandError::system_fault(
                        "runtime_stream_sequence_invalid",
                        format!(
                            "Cadence rejected detached supervisor event sequence {sequence} because the prior bridged sequence was {}.",
                            attach_state.last_sequence
                        ),
                    ),
                    last_sequence: attach_state.last_sequence,
                }));
            }

            let item = map_supervisor_event_to_stream_item(request, sequence, created_at, item)
                .map_err(|error| StreamExit::Failed(StreamFailure {
                    error,
                    last_sequence: attach_state.last_sequence,
                }))?;
            if let Some(action_id) = item.action_id.as_ref() {
                attach_state.action_required_ids.insert(action_id.clone());
            }
            emit_item_if_requested(channel, request, lease, item)?;
            attach_state.last_sequence = sequence;
            Ok(attach_state)
        }
        SupervisorControlResponse::Error {
            code,
            message,
            retryable,
            ..
        } => Err(StreamExit::Failed(StreamFailure {
            error: if retryable {
                CommandError::retryable(code, message)
            } else {
                CommandError::user_fixable(code, message)
            },
            last_sequence: attach_state.last_sequence,
        })),
        other => Err(StreamExit::Failed(StreamFailure {
            error: CommandError::system_fault(
                "runtime_stream_contract_invalid",
                format!(
                    "Cadence expected a detached supervisor event frame but received `{other:?}` instead."
                ),
            ),
            last_sequence: attach_state.last_sequence,
        })),
    }
}

fn read_supervisor_response(
    reader: &mut BufReader<TcpStream>,
) -> Result<Option<SupervisorControlResponse>, ReadSupervisorResponseError> {
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => Ok(None),
        Ok(_) => {
            if line.len() > PROTOCOL_LINE_LIMIT {
                return Err(ReadSupervisorResponseError::Decode(
                    "line exceeded protocol limit".into(),
                ));
            }

            serde_json::from_str(line.trim())
                .map(Some)
                .map_err(|error| ReadSupervisorResponseError::Decode(error.to_string()))
        }
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) =>
        {
            Err(ReadSupervisorResponseError::Timeout)
        }
        Err(error) => Err(ReadSupervisorResponseError::Io(error)),
    }
}

fn write_json_line<W: Write>(
    writer: &mut W,
    value: &SupervisorControlRequest,
) -> Result<(), std::io::Error> {
    serde_json::to_writer(&mut *writer, value).map_err(std::io::Error::other)?;
    writer.write_all(b"\n")?;
    writer.flush()
}
