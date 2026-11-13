use openkdx_proto::MessageType;
use openkdx_proto::handshake::ProofStatus;
use openkdx_proto::wire::DecodeError;

/// Everything that can go wrong between the client library and a server.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Codec(#[from] openkdx_codec::CodecError),
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error(
        "no KDX challenge table; run `openkdx-table fetch` or `openkdx-table import <KDX binary>`"
    )]
    NoTable,
    #[error("login failed (server proof status {0:?})")]
    LoginFailed(ProofStatus),
    #[error("login name longer than 31 bytes")]
    LoginTooLong,
    #[error("the server's challenge offset {0} is past the end of the table")]
    BadChallenge(u16),
    #[error("the server's proof does not match the account key")]
    ProofMismatch,
    #[error("timed out waiting for the server")]
    Timeout,
    #[error("disconnected")]
    Disconnected,
    #[error("server error {code:#x}{} in reply to {request}", name.map(|n| format!(" ({n})")).unwrap_or_default())]
    Server {
        code: u32,
        name: Option<&'static str>,
        request: u32,
    },
    #[error("expected {expected}, got {got}")]
    Unexpected {
        expected: MessageType,
        got: MessageType,
    },
    #[error("transfer: {0}")]
    Transfer(&'static str),
}

pub type Result<T> = std::result::Result<T, ClientError>;
