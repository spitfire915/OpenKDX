//! Framing for the handshake: a hello, then a 44-byte proof.

use bytes::{Buf, BufMut, BytesMut};
use openkdx_proto::handshake::{HandshakeError, Hello, Proof};
use tokio_util::codec::{Decoder, Encoder};

use crate::CodecError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeFrame {
    Hello(Hello),
    Proof(Proof),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    Hello,
    Proof,
    Done,
}

/// Decodes the peer's hello and proof in order, and encodes ours.
#[derive(Debug, Clone)]
pub struct HandshakeCodec {
    expect: Expect,
    /// The XOR key our hello goes out under; the caller picks a random one.
    pub hello_key: u32,
}

impl HandshakeCodec {
    pub fn new(hello_key: u32) -> Self {
        HandshakeCodec {
            expect: Expect::Hello,
            hello_key,
        }
    }

    /// Both frames have been decoded.
    pub fn is_done(&self) -> bool {
        self.expect == Expect::Done
    }
}

impl Decoder for HandshakeCodec {
    type Item = HandshakeFrame;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<HandshakeFrame>, CodecError> {
        match self.expect {
            Expect::Hello => match Hello::decode(src) {
                Ok((hello, used)) => {
                    src.advance(used);
                    self.expect = Expect::Proof;
                    Ok(Some(HandshakeFrame::Hello(hello)))
                }
                Err(HandshakeError::Incomplete { .. }) => Ok(None),
                Err(e) => Err(e.into()),
            },
            Expect::Proof => {
                if src.len() < Proof::LEN {
                    return Ok(None);
                }
                let bytes: [u8; 44] = src[..Proof::LEN].try_into().expect("44 bytes");
                let proof = Proof::decode(&bytes)?;
                src.advance(Proof::LEN);
                self.expect = Expect::Done;
                Ok(Some(HandshakeFrame::Proof(proof)))
            }
            Expect::Done => Err(CodecError::Protocol(
                "handshake is complete; switch to the session codec",
            )),
        }
    }
}

impl Encoder<HandshakeFrame> for HandshakeCodec {
    type Error = CodecError;

    fn encode(&mut self, item: HandshakeFrame, dst: &mut BytesMut) -> Result<(), CodecError> {
        match item {
            HandshakeFrame::Hello(h) => dst.put_slice(&h.encode(self.hello_key)),
            HandshakeFrame::Proof(p) => dst.put_slice(&p.encode()),
        }
        Ok(())
    }
}
