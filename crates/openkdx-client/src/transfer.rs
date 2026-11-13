//! File transfers on their own connection: one download or upload at a
//! time, driven directly on the codec's data mode (reference `transfers.md`).

use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use openkdx_codec::{Event as CodecEvent, Item, SessionCodec};
use openkdx_proto::msg::common::FilePath;
use openkdx_proto::msg::error_name;
use openkdx_proto::msg::files::{
    DownloadFile, DownloadInfo, FileStrings, UploadFile, UploadFileData, UploadReply,
};
use openkdx_proto::session::{Header, TRANSFER_RECV_CAP};
use openkdx_proto::{Message, MessageType, Payload};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, FramedParts};

use crate::handshake::client_handshake;
use crate::session::Session;
use crate::txid::TxidSource;
use crate::{ClientError, Result};

/// Bytes moved so far and the fork's total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub done: u64,
    pub total: u64,
}

const CHUNK: usize = 32 * 1024;

pub struct Transfer {
    framed: Framed<TcpStream, SessionCodec>,
    txids: TxidSource,
    timeout: Duration,
}

impl Transfer {
    /// Open a transfer connection attached to `session`: the login is
    /// `\0` + user ID and the key is the 105 session key (`[O files-basic 21, 25]`).
    pub async fn open(session: &Session) -> Result<Transfer> {
        let timeout = session.connection().timeout();
        let mut login = vec![0u8];
        login.extend_from_slice(&session.user_id().to_be_bytes());
        let key = session.session_key();
        let mut stream = tokio::time::timeout(timeout, TcpStream::connect(session.transfer_addr()))
            .await
            .map_err(|_| ClientError::Timeout)??;
        let out = tokio::time::timeout(
            timeout,
            client_handshake(&mut stream, &login, &key, session.table()),
        )
        .await
        .map_err(|_| ClientError::Timeout)??;
        let mut parts =
            FramedParts::new::<Item>(stream, SessionCodec::new(&key, TRANSFER_RECV_CAP));
        parts.read_buf = out.read_buf;
        Ok(Transfer {
            framed: Framed::from_parts(parts),
            txids: TxidSource::new_random(),
            timeout,
        })
    }

    async fn next(&mut self) -> Result<CodecEvent> {
        match tokio::time::timeout(self.timeout, self.framed.next()).await {
            Err(_) => Err(ClientError::Timeout),
            Ok(None) => Err(ClientError::Disconnected),
            Ok(Some(Err(e))) => Err(e.into()),
            Ok(Some(Ok(ev))) => Ok(ev),
        }
    }

    async fn send(&mut self, item: Item) -> Result<()> {
        self.framed.send(item).await.map_err(Into::into)
    }

    /// Send a request and wait for its reply (reply flag + txid); a `Status`
    /// error becomes `Server`.
    async fn request<T: Payload, R: Payload>(&mut self, req: &T) -> Result<(u32, R)> {
        let txid = self.txids.next();
        let payload = req.to_bytes();
        let header = Header::request(txid, T::KIND.0, payload.len() as u64);
        self.send(Item::Message {
            header,
            payload: Bytes::from(payload),
        })
        .await?;
        loop {
            match self.next().await? {
                CodecEvent::Message { header, payload } if header.reply && header.txid == txid => {
                    let kind = MessageType(header.kind);
                    if kind == R::KIND {
                        return Ok((txid, R::decode(&payload)?));
                    }
                    if let Ok(Message::Status(s)) = Message::decode(kind, &payload)
                        && s.is_error()
                    {
                        return Err(ClientError::Server {
                            code: s.code,
                            name: error_name(s.code),
                            request: T::KIND.0,
                        });
                    }
                    return Err(ClientError::Unexpected {
                        expected: R::KIND,
                        got: kind,
                    });
                }
                CodecEvent::Message { .. } => {}
                _ => return Err(ClientError::Protocol("transfer data before the reply")),
            }
        }
    }

    /// Wait for the next data header.
    async fn data_start(&mut self) -> Result<Header> {
        loop {
            match self.next().await? {
                CodecEvent::DataStart(h) => return Ok(h),
                CodecEvent::Message { header, payload } => {
                    if let Ok(Message::Status(s)) =
                        Message::decode(MessageType(header.kind), &payload)
                        && s.is_error()
                    {
                        return Err(ClientError::Server {
                            code: s.code,
                            name: error_name(s.code),
                            request: DownloadFile::KIND.0,
                        });
                    }
                }
                _ => return Err(ClientError::Protocol("data without a data header")),
            }
        }
    }

    /// 412 → 413 → 414 (data fork) → 414 (resource fork). With `resume`,
    /// asks from the local file's length; the server rounds the offset down
    /// to 0x1000, so the local file is rewritten from where the data starts
    /// (`data_size - header.len`). A length-0 fork leaves the file as it is.
    pub async fn download(
        &mut self,
        path: &FilePath,
        dest: &Path,
        resume: bool,
        mut on_progress: impl FnMut(Progress),
    ) -> Result<DownloadInfo> {
        let existing = if resume {
            tokio::fs::metadata(dest)
                .await
                .map(|m| m.len())
                .unwrap_or(0)
        } else {
            0
        };
        let (_txid, info) = self
            .request::<_, DownloadInfo>(&DownloadFile {
                data_offset: existing,
                rsrc_offset: u64::MAX,
                ignored: [0; 7],
                options: 0,
                path: path.clone(),
            })
            .await?;

        let header = self.data_start().await?;
        let start = info
            .data_size
            .checked_sub(header.len)
            .ok_or(ClientError::Transfer("data fork longer than the file"))?;
        if start > existing {
            return Err(ClientError::Transfer("server resumed past the local data"));
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(!resume)
            .open(dest)
            .await?;
        if header.len > 0 || !resume {
            file.set_len(start).await?;
            file.seek(SeekFrom::Start(start)).await?;
        }
        let mut done = start;
        on_progress(Progress {
            done,
            total: info.data_size,
        });
        loop {
            match self.next().await? {
                CodecEvent::Data(chunk) => {
                    file.write_all(&chunk).await?;
                    done += chunk.len() as u64;
                    on_progress(Progress {
                        done,
                        total: info.data_size,
                    });
                }
                CodecEvent::DataEnd => break,
                _ => return Err(ClientError::Protocol("message inside transfer data")),
            }
        }
        file.flush().await?;
        drop(file);

        // The resource fork: keep it beside the file only if there is one.
        let rsrc = self.data_start().await?;
        let mut rsrc_file = if rsrc.len > 0 {
            let mut p = dest.as_os_str().to_owned();
            p.push(".rsrc");
            Some(File::create(PathBuf::from(p)).await?)
        } else {
            None
        };
        loop {
            match self.next().await? {
                CodecEvent::Data(chunk) => {
                    if let Some(f) = rsrc_file.as_mut() {
                        f.write_all(&chunk).await?;
                    }
                }
                CodecEvent::DataEnd => break,
                _ => return Err(ClientError::Protocol("message inside transfer data")),
            }
        }
        Ok(info)
    }

    /// `download` into `dir`, named after the last path component.
    pub async fn download_to_dir(
        &mut self,
        path: &FilePath,
        dir: &Path,
        resume: bool,
        on_progress: impl FnMut(Progress),
    ) -> Result<PathBuf> {
        let name = path
            .names
            .last()
            .ok_or(ClientError::Transfer("cannot download the base folder"))?;
        let dest = dir.join(String::from_utf8_lossy(&name.name).as_ref());
        self.download(path, &dest, resume, on_progress).await?;
        Ok(dest)
    }

    /// 415 → 416 → 417 (data fork, from 416's offset) → 417 (empty resource
    /// fork) → 418. `stop_after` (tests and lab scenarios) drops the
    /// connection after that many data bytes, leaving a partial upload.
    pub async fn upload(
        &mut self,
        local: &Path,
        folder: &FilePath,
        name: &[u8],
        resume: bool,
        mut on_progress: impl FnMut(Progress),
        stop_after: Option<u64>,
    ) -> Result<()> {
        let mut file = File::open(local).await?;
        let size = file.metadata().await?.len();
        let (txid, reply) = self
            .request::<_, UploadReply>(&UploadFile {
                data_size: size,
                rsrc_size: 0,
                ignored_10: [0; 7],
                options: 0,
                ignored_18: [0; 24],
                ignored_30: [0; 22],
                strings: FileStrings {
                    name: name.to_vec(),
                    second: kind_for(name),
                    third: Vec::new(),
                    fourth: Vec::new(),
                },
                folder: folder.clone(),
                rest: Vec::new(),
            })
            .await?;
        let offset = if resume {
            reply.data_offset.min(size)
        } else {
            0
        };
        if !resume && reply.data_offset != 0 {
            return Err(ClientError::Transfer(
                "the server holds a partial upload of this name; use resume",
            ));
        }
        let remaining = size - offset;
        // The data fork header answers 415, as the server's 414 answers 412.
        self.send(Item::DataHeader(Header::reply(
            txid,
            UploadFileData::KIND.0,
            remaining,
        )))
        .await?;
        file.seek(SeekFrom::Start(offset)).await?;
        let mut done = offset;
        let mut buf = vec![0u8; CHUNK];
        on_progress(Progress { done, total: size });
        while done < size {
            let n = file.read(&mut buf).await?;
            if n == 0 {
                return Err(ClientError::Transfer("local file shrank during upload"));
            }
            self.send(Item::Data(Bytes::copy_from_slice(&buf[..n])))
                .await?;
            done += n as u64;
            on_progress(Progress { done, total: size });
            if let Some(limit) = stop_after
                && done - offset >= limit
            {
                let _ = self.framed.get_mut().shutdown().await;
                return Err(ClientError::Transfer("stopped on request"));
            }
        }
        self.send(Item::DataHeader(Header::request(
            self.txids.next(),
            UploadFileData::KIND.0,
            0,
        )))
        .await?;
        loop {
            match self.next().await? {
                CodecEvent::Message { header, payload } => {
                    let kind = MessageType(header.kind);
                    if kind == MessageType::UPLOAD_FILE_COMPLETE {
                        return Ok(());
                    }
                    if let Ok(Message::Status(s)) = Message::decode(kind, &payload)
                        && s.is_error()
                    {
                        return Err(ClientError::Server {
                            code: s.code,
                            name: error_name(s.code),
                            request: UploadFile::KIND.0,
                        });
                    }
                }
                _ => return Err(ClientError::Protocol("data while waiting for 418")),
            }
        }
    }

    pub async fn close(mut self) {
        let _ = self.framed.close().await;
    }
}

/// The type string for an upload: `exttype/<ext>` as the server itself
/// labels files (`[O files-basic 16]`), empty without an extension.
pub fn kind_for(name: &[u8]) -> Vec<u8> {
    match name.iter().rposition(|&b| b == b'.') {
        Some(i) if i + 1 < name.len() && i > 0 => {
            let mut out = b"exttype/".to_vec();
            out.extend(name[i + 1..].iter().map(|b| b.to_ascii_lowercase()));
            out
        }
        _ => Vec::new(),
    }
}
