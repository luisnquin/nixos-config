use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

// Re-export the wire types so the rest of the app has a single import path and
// never depends on greetd_ipc's module layout directly.
pub use greetd_ipc::{AuthMessageType, ErrorType, Request, Response};
use greetd_ipc::codec::TokioCodec;

// greetd answers exactly one Response per Request, so real socket and mock can
// share one ordered send/receive pipe.
pub struct Channels {
    pub req_tx: mpsc::Sender<Request>,
    pub resp_rx: mpsc::Receiver<Response>,
}

// Owns the socket on a background task; all IO stays here, the UI only sees channels.
pub async fn spawn_real(sock: &Path) -> Result<Channels> {
    let stream = UnixStream::connect(sock)
        .await
        .with_context(|| format!("connecting to greetd socket {}", sock.display()))?;
    let (mut rd, mut wr) = stream.into_split();

    let (req_tx, mut req_rx) = mpsc::channel::<Request>(8);
    let (resp_tx, resp_rx) = mpsc::channel::<Response>(8);

    tokio::spawn(async move {
        while let Some(req) = req_rx.recv().await {
            if req.write_to(&mut wr).await.is_err() {
                break;
            }
            match Response::read_from(&mut rd).await {
                Ok(resp) => {
                    if resp_tx.send(resp).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    Ok(Channels { req_tx, resp_rx })
}

// In-process fake greetd for --demo and tests: one password prompt, `demo` or
// empty succeeds, anything else fails. No PAM, no session launch.
pub fn spawn_mock() -> Channels {
    let (req_tx, mut req_rx) = mpsc::channel::<Request>(8);
    let (resp_tx, resp_rx) = mpsc::channel::<Response>(8);

    tokio::spawn(async move {
        // Track greetd's "configuring" slot so demo reproduces the real IPC state
        // machine, including rejecting a second CreateSession without a cancel.
        let mut configuring = false;
        while let Some(req) = req_rx.recv().await {
            let resp = mock_reply(&mut configuring, &req);
            // Small latency so the "authenticating" phase is actually visible.
            tokio::time::sleep(Duration::from_millis(180)).await;
            if resp_tx.send(resp).await.is_err() {
                break;
            }
        }
    });

    Channels { req_tx, resp_rx }
}

// Mirrors greetd's session lifecycle: one session may be "configuring" at a time,
// a failed password leaves it configuring (greetd does not self-cancel), and only
// CancelSession/StartSession clear it.
pub fn mock_reply(configuring: &mut bool, req: &Request) -> Response {
    match req {
        Request::CreateSession { .. } => {
            if *configuring {
                return Response::Error {
                    error_type: ErrorType::Error,
                    description: "a session is already being configured".into(),
                };
            }
            *configuring = true;
            Response::AuthMessage {
                auth_message_type: AuthMessageType::Secret,
                auth_message: "Password: ".into(),
            }
        }
        // On auth failure the session stays configured, exactly like greetd.
        Request::PostAuthMessageResponse { response } => match response.as_deref() {
            Some("demo") | Some("") | None => Response::Success,
            _ => Response::Error {
                error_type: ErrorType::AuthError,
                description: "authentication failed".into(),
            },
        },
        Request::StartSession { .. } | Request::CancelSession => {
            *configuring = false;
            Response::Success
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_session_prompts_for_password() {
        let mut configuring = false;
        let r = mock_reply(
            &mut configuring,
            &Request::CreateSession {
                username: "0xc000022070".into(),
            },
        );
        assert!(matches!(
            r,
            Response::AuthMessage {
                auth_message_type: AuthMessageType::Secret,
                ..
            }
        ));
    }

    #[test]
    fn correct_password_succeeds_wrong_fails() {
        let mut configuring = true;
        let ok = mock_reply(
            &mut configuring,
            &Request::PostAuthMessageResponse {
                response: Some("demo".into()),
            },
        );
        assert!(matches!(ok, Response::Success));

        let bad = mock_reply(
            &mut configuring,
            &Request::PostAuthMessageResponse {
                response: Some("nope".into()),
            },
        );
        assert!(matches!(
            bad,
            Response::Error {
                error_type: ErrorType::AuthError,
                ..
            }
        ));
    }

    #[test]
    fn second_create_without_cancel_is_rejected_but_cancel_recovers() {
        let mut configuring = false;
        mock_reply(
            &mut configuring,
            &Request::CreateSession {
                username: "u".into(),
            },
        );
        // A retry without cancelling first is rejected, mirroring greetd.
        let dup = mock_reply(
            &mut configuring,
            &Request::CreateSession {
                username: "u".into(),
            },
        );
        assert!(matches!(dup, Response::Error { .. }));

        // After a cancel the slot frees and a fresh create prompts again.
        mock_reply(&mut configuring, &Request::CancelSession);
        let fresh = mock_reply(
            &mut configuring,
            &Request::CreateSession {
                username: "u".into(),
            },
        );
        assert!(matches!(fresh, Response::AuthMessage { .. }));
    }
}
