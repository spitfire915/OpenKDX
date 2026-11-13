//! The client side of the hello/proof exchange on any async stream.

use bytes::BytesMut;
use futures_util::{SinkExt, StreamExt};
use openkdx_codec::{HandshakeCodec, HandshakeFrame};
use openkdx_proto::handshake::{Hello, Proof, ProofStatus, SubProtocol, proof_auth};
use openkdx_proto::{AccountKey, ChallengeTable};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

use crate::{ClientError, Result};

/// The longest login name the server keeps.
pub const MAX_LOGIN: usize = 31;

#[derive(Debug)]
pub struct HandshakeOutcome {
    pub server_hello: Hello,
    pub server_proof: Proof,
    /// Bytes read past the proof; hand them to the session codec.
    pub read_buf: BytesMut,
}

/// Send our hello and proof as a registered Client 1.600 and check the
/// server's. `login` is the account name, empty for guest, or `\0` + user ID
/// to attach a transfer connection.
pub async fn client_handshake<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    login: &[u8],
    key: &AccountKey,
    table: &ChallengeTable,
) -> Result<HandshakeOutcome> {
    if login.len() > MAX_LOGIN {
        return Err(ClientError::LoginTooLong);
    }
    let nonce: [u8; 12] = rand::random();
    let hello = Hello {
        sub_protocol: SubProtocol::Kdx,
        version: 1,
        sub_version: 0x10,
        challenge: rand::random_range(0..=table.max_offset()),
        nonce,
        name: login.to_vec(),
        pad_len: 0,
        skipped_len: 0,
        pad: Vec::new(),
        skipped: Vec::new(),
        ignored: [0; 4],
        padding: Vec::new(),
    };
    let mut framed = Framed::new(stream, HandshakeCodec::new(rand::random()));
    framed.send(HandshakeFrame::Hello(hello)).await?;
    let server_hello = match framed.next().await {
        Some(Ok(HandshakeFrame::Hello(h))) => h,
        Some(Ok(HandshakeFrame::Proof(_))) => {
            return Err(ClientError::Protocol("proof before hello"));
        }
        Some(Err(e)) => return Err(e.into()),
        None => return Err(ClientError::Disconnected),
    };
    let proof = Proof::compute(key, &server_hello.nonce, table, server_hello.challenge)
        .ok_or(ClientError::BadChallenge(server_hello.challenge))?;
    framed.send(HandshakeFrame::Proof(proof)).await?;
    let server_proof = match framed.next().await {
        Some(Ok(HandshakeFrame::Proof(p))) => p,
        Some(Ok(HandshakeFrame::Hello(_))) => {
            return Err(ClientError::Protocol("second hello"));
        }
        Some(Err(e)) => return Err(e.into()),
        // Server 1.620 just closes on a bad proof.
        None => return Err(ClientError::LoginFailed(ProofStatus::Ok)),
    };
    if server_proof.status != ProofStatus::Ok {
        return Err(ClientError::LoginFailed(server_proof.status));
    }
    if server_proof.auth != proof_auth(key, &nonce) {
        return Err(ClientError::ProofMismatch);
    }
    let parts = framed.into_parts();
    Ok(HandshakeOutcome {
        server_hello,
        server_proof,
        read_buf: parts.read_buf,
    })
}
