//! Framing for the session: encrypted 16-byte header blocks and padded
//! payload blocks, with a streaming mode for transfer data (414 and 417).

use bytes::{Buf, BufMut, Bytes, BytesMut};
use openkdx_proto::AccountKey;
use openkdx_proto::message_type::MessageType;
use openkdx_proto::session::{Header, Scramble, SessionCipher, SessionError, padding_bytes};
use tokio_util::codec::{Decoder, Encoder};

use crate::CodecError;

const BLOCK: usize = 16;

/// What the decoder yields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Message {
        header: Header,
        payload: Bytes,
    },
    /// A 414 or 417 header: `header.len` bytes of transfer data follow as
    /// [`Event::Data`] chunks, then [`Event::DataEnd`].
    DataStart(Header),
    Data(Bytes),
    DataEnd,
}

/// What the encoder takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Message {
        header: Header,
        payload: Bytes,
    },
    /// A 414 or 417 header; follow it with [`Item::Data`] totalling `header.len` bytes.
    DataHeader(Header),
    /// Any length; the codec buffers to 16-byte blocks and pads the last one.
    Data(Bytes),
}

#[derive(Debug, Clone)]
enum DecodeState {
    Header,
    Payload(Header),
    /// Transfer data: bytes of the fork still to deliver, and padded bytes
    /// still to consume.
    Data {
        remaining: u64,
        padded_left: usize,
    },
}

#[derive(Debug, Clone)]
struct EncodeData {
    txid: u32,
    remaining: u64,
    declared: u64,
    pending: Vec<u8>,
}

/// The session codec. `max_len` is the receive cap for normal messages.
#[derive(Clone)]
pub struct SessionCodec {
    cipher: SessionCipher,
    max_len: u64,
    state: DecodeState,
    encode_data: Option<EncodeData>,
}

impl SessionCodec {
    pub fn new(key: &AccountKey, max_len: u64) -> Self {
        Self::with_cipher(SessionCipher::new(key), max_len)
    }

    pub fn with_cipher(cipher: SessionCipher, max_len: u64) -> Self {
        SessionCodec {
            cipher,
            max_len,
            state: DecodeState::Header,
            encode_data: None,
        }
    }

    /// A codec for a GConsole peer, which uses the other header scramble.
    pub fn with_scramble(mut self, scramble: Scramble) -> Self {
        self.cipher = self.cipher.with_scramble(scramble);
        self
    }

    pub fn into_parts(self) -> (SessionCipher, u64) {
        (self.cipher, self.max_len)
    }

    fn is_data_kind(kind: u32) -> bool {
        kind == MessageType::DOWNLOAD_FILE_DATA.0 || kind == MessageType::UPLOAD_FILE_DATA.0
    }
}

impl Decoder for SessionCodec {
    type Item = Event;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Event>, CodecError> {
        loop {
            match self.state.clone() {
                DecodeState::Header => {
                    if src.len() < BLOCK {
                        return Ok(None);
                    }
                    let block: &[u8; 16] = src[..BLOCK].try_into().expect("16 bytes");
                    let header = self.cipher.open_header_unbounded(block)?;
                    if Self::is_data_kind(header.kind) {
                        src.advance(BLOCK);
                        let padded = header.padded_len();
                        self.state = if padded == 0 {
                            DecodeState::Header
                        } else {
                            DecodeState::Data {
                                remaining: header.len,
                                padded_left: padded,
                            }
                        };
                        // An empty fork ends at once; the caller still sees DataEnd.
                        if padded == 0 {
                            self.state = DecodeState::Data {
                                remaining: 0,
                                padded_left: 0,
                            };
                        }
                        return Ok(Some(Event::DataStart(header)));
                    }
                    if header.len > self.max_len || header.len >> 32 != 0 {
                        return Err(SessionError::LengthTooLong {
                            len: header.len,
                            cap: self.max_len,
                        }
                        .into());
                    }
                    src.advance(BLOCK);
                    src.reserve(header.padded_len());
                    self.state = DecodeState::Payload(header);
                }
                DecodeState::Payload(header) => {
                    let padded = header.padded_len();
                    if src.len() < padded {
                        return Ok(None);
                    }
                    let payload = self.cipher.open_payload(&header, &src[..padded])?;
                    src.advance(padded);
                    self.state = DecodeState::Header;
                    return Ok(Some(Event::Message {
                        header,
                        payload: Bytes::from(payload),
                    }));
                }
                DecodeState::Data {
                    remaining,
                    padded_left,
                } => {
                    if padded_left == 0 {
                        self.state = DecodeState::Header;
                        return Ok(Some(Event::DataEnd));
                    }
                    let whole_blocks = (src.len() / BLOCK) * BLOCK;
                    let take = whole_blocks.min(padded_left);
                    if take == 0 {
                        return Ok(None);
                    }
                    let mut chunk = src.split_to(take).to_vec();
                    self.cipher.decrypt_blocks(&mut chunk);
                    let deliver = (remaining.min(take as u64)) as usize;
                    chunk.truncate(deliver);
                    self.state = DecodeState::Data {
                        remaining: remaining - deliver as u64,
                        padded_left: padded_left - take,
                    };
                    return Ok(Some(Event::Data(Bytes::from(chunk))));
                }
            }
        }
    }
}

impl Encoder<Item> for SessionCodec {
    type Error = CodecError;

    fn encode(&mut self, item: Item, dst: &mut BytesMut) -> Result<(), CodecError> {
        match item {
            Item::Message { header, payload } => {
                if self.encode_data.as_ref().is_some_and(|d| d.remaining > 0) {
                    return Err(CodecError::Protocol(
                        "a message cannot interrupt transfer data",
                    ));
                }
                self.encode_data = None;
                dst.put_slice(&self.cipher.seal(header, &payload));
            }
            Item::DataHeader(header) => {
                if self.encode_data.as_ref().is_some_and(|d| d.remaining > 0) {
                    return Err(CodecError::Protocol(
                        "a data header cannot interrupt transfer data",
                    ));
                }
                dst.put_slice(&self.cipher.seal_header(header));
                self.encode_data = Some(EncodeData {
                    txid: header.txid,
                    remaining: header.len,
                    declared: header.len,
                    pending: Vec::new(),
                });
            }
            Item::Data(bytes) => {
                let Some(data) = self.encode_data.as_mut() else {
                    return Err(CodecError::Protocol("transfer data without a data header"));
                };
                if (bytes.len() as u64) > data.remaining {
                    return Err(CodecError::DataOverrun {
                        sent: data.declared - data.remaining + bytes.len() as u64,
                        declared: data.declared,
                    });
                }
                data.pending.extend_from_slice(&bytes);
                data.remaining -= bytes.len() as u64;
                let last = data.remaining == 0;
                let full = (data.pending.len() / BLOCK) * BLOCK;
                let mut out = if last {
                    std::mem::take(&mut data.pending)
                } else {
                    data.pending.drain(..full).collect::<Vec<u8>>()
                };
                if last && out.len() % BLOCK != 0 {
                    let pad = padding_bytes(data.txid);
                    let start = out.len();
                    let target = start.div_ceil(BLOCK) * BLOCK;
                    for i in start..target {
                        out.push(pad[i % 4]);
                    }
                }
                self.cipher.encrypt_blocks(&mut out);
                dst.put_slice(&out);
                if last {
                    self.encode_data = None;
                }
            }
        }
        Ok(())
    }
}
