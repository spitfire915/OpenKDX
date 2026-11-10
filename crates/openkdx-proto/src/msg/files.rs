//! Messages 400–453: file listings, folders, downloads, uploads, volumes,
//! catalog, and search. See the reference spec's `messages/files.md`.

use crate::message_type::MessageType;
use crate::msg::common::{FilePath, Time1900};
use crate::msg::{Payload, empty_payload};
use crate::wire::{DecodeError, Reader, Writer};

/// Reads a `FilePath` that an earlier length field sizes in bytes.
fn sized_path(r: &mut Reader, len: usize) -> Result<FilePath, DecodeError> {
    let bytes = r.bytes(len)?;
    FilePath::decode(&bytes)
}

/// A message whose payload is one `FilePath`, empty for the root.
macro_rules! path_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq, Default)]
            pub struct $name {
                pub path: FilePath,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    Ok($name { path: FilePath::decode(bytes)? })
                }

                fn encode(&self, w: &mut Writer) {
                    w.bytes(&self.path.to_bytes());
                }
            }
        )*
    };
}

path_payload! {
    /// 400: reply is 401.
    GetFileList = GET_FILE_LIST;
    /// 409: reply is 410, or 422 for a volume.
    GetFileFolderInfo = GET_FILE_INFO;
}

/// A request Server 1.620 has no handler for; the bytes are kept as they are.
macro_rules! unhandled_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq, Default)]
            pub struct $name {
                pub ignored: Vec<u8>,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    Ok($name { ignored: bytes.to_vec() })
                }

                fn encode(&self, w: &mut Writer) {
                    w.bytes(&self.ignored);
                }
            }
        )*
    };
}

unhandled_payload! {
    /// 404: unhandled by Server 1.620.
    DeleteFileFolder = DELETE_FILE;
    /// 406: unhandled by Server 1.620.
    RenameFileFolder = RENAME_FILE;
    /// 408: unhandled by Server 1.620.
    CopyFileFolder = COPY_FILE;
}

empty_payload! {
    /// 418: both forks of an upload are complete.
    UploadFileComplete = UPLOAD_FILE_COMPLETE;
    /// 421: reply is 1 Status 0x1a5.
    EmptyTrash = EMPTY_TRASH;
}

/// One entry of 401 File List.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileListEntry {
    /// 0x1000 for folders.
    pub size: u64,
    pub created: Time1900,
    pub modified: Time1900,
    pub ignored_18: [u8; 8],
    /// 1 for a file, 0x01000002 for a folder.
    pub attributes: u32,
    pub ignored_24: [u8; 10],
    /// 0xffff for folders, 0 for a file.
    pub ignored_2e: u16,
    /// The folder access item's icon: 0x2c Dropbox, 0x2b Uploads, 0 for a file.
    pub access_item: u16,
    pub ignored_32: [u8; 2],
    pub ignored_34: u8,
    pub ignored_37: u8,
    pub name: Vec<u8>,
    /// Empty for folders; `exttype/bin` for a `.bin` file.
    pub kind: Vec<u8>,
    /// Uninitialized on the wire; kept as captured.
    pub padding: Vec<u8>,
}

impl FileListEntry {
    pub const ATTR_FILE: u32 = 1;
    pub const ATTR_FOLDER: u32 = 0x0100_0002;

    pub fn is_folder(&self) -> bool {
        self.attributes & 2 != 0
    }

    fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        let start = r.pos();
        let size = r.u64()?;
        let created = Time1900::decode(r)?;
        let modified = Time1900::decode(r)?;
        let ignored_18: [u8; 8] = r.array()?;
        let attributes = r.u32()?;
        let ignored_24: [u8; 10] = r.array()?;
        let ignored_2e = r.u16()?;
        let access_item = r.u16()?;
        let ignored_32: [u8; 2] = r.array()?;
        let ignored_34 = r.u8()?;
        let name_len = usize::from(r.u8()?);
        let kind_len = usize::from(r.u8()?);
        let ignored_37 = r.u8()?;
        Ok(FileListEntry {
            size,
            created,
            modified,
            ignored_18,
            attributes,
            ignored_24,
            ignored_2e,
            access_item,
            ignored_32,
            ignored_34,
            ignored_37,
            name: r.bytes(name_len)?,
            kind: r.bytes(kind_len)?,
            padding: r.pad4(start)?,
        })
    }

    fn encode(&self, w: &mut Writer) {
        let start = w.pos();
        w.u64(self.size);
        self.created.encode(w);
        self.modified.encode(w);
        w.bytes(&self.ignored_18);
        w.u32(self.attributes);
        w.bytes(&self.ignored_24);
        w.u16(self.ignored_2e);
        w.u16(self.access_item);
        w.bytes(&self.ignored_32);
        w.u8(self.ignored_34);
        w.u8(self.name.len() as u8);
        w.u8(self.kind.len() as u8);
        w.u8(self.ignored_37);
        w.bytes(&self.name);
        w.bytes(&self.kind);
        w.pad4(start, &self.padding);
    }
}

/// 401: reply to 400. An empty folder gets an empty payload.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileList {
    pub entries: Vec<FileListEntry>,
}

impl Payload for FileList {
    const KIND: MessageType = MessageType::FILE_LIST;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.is_empty() {
            return Ok(FileList::default());
        }
        let mut r = Reader::new(bytes);
        let count = r.u32()? as usize;
        let mut entries = Vec::with_capacity(count.min(4096));
        for _ in 0..count {
            entries.push(FileListEntry::decode(&mut r)?);
        }
        r.finish()?;
        Ok(FileList { entries })
    }

    fn encode(&self, w: &mut Writer) {
        if self.entries.is_empty() {
            return;
        }
        w.u32(self.entries.len() as u32);
        for e in &self.entries {
            e.encode(w);
        }
    }
}

/// 402: reply is 1 Status 0x192.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreateFolder {
    pub ignored: [u8; 19],
    /// If empty, the last name in `path` is the new folder.
    pub name: Vec<u8>,
    /// A folder access tag such as `DB`; needs Modify Folder Access.
    pub access: Vec<u8>,
    pub owner: Vec<u8>,
    /// The parent folder.
    pub path: FilePath,
}

impl Payload for CreateFolder {
    const KIND: MessageType = MessageType::CREATE_FOLDER;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let ignored: [u8; 19] = r.array()?;
        let access_len = usize::from(r.u8()?);
        let path_len = usize::from(r.u16()?);
        let name_len = usize::from(r.u8()?);
        let owner_len = usize::from(r.u8()?);
        let out = CreateFolder {
            ignored,
            name: r.bytes(name_len)?,
            access: r.bytes(access_len)?,
            owner: r.bytes(owner_len)?,
            path: sized_path(&mut r, path_len)?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        let path = self.path.to_bytes();
        w.bytes(&self.ignored);
        w.u8(self.access.len() as u8);
        w.u16(path.len() as u16);
        w.u8(self.name.len() as u8);
        w.u8(self.owner.len() as u8);
        w.bytes(&self.name);
        w.bytes(&self.access);
        w.bytes(&self.owner);
        w.bytes(&path);
    }
}

/// The layout of 403 and 407: three delimited paths.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MoveItems {
    /// The folder the items, or their aliases, go to.
    pub destination: FilePath,
    /// The folder that holds the items.
    pub source: FilePath,
    /// The items' names, as a name list.
    pub names: FilePath,
}

impl MoveItems {
    fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        Ok(MoveItems {
            destination: FilePath::decode_delimited(r)?,
            source: FilePath::decode_delimited(r)?,
            names: FilePath::decode_delimited(r)?,
        })
    }

    fn encode(&self, w: &mut Writer) {
        self.destination.encode_delimited(w);
        self.source.encode_delimited(w);
        self.names.encode_delimited(w);
    }
}

macro_rules! move_items_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq, Default)]
            pub struct $name {
                pub items: MoveItems,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    let mut r = Reader::new(bytes);
                    let items = MoveItems::decode(&mut r)?;
                    r.finish()?;
                    Ok($name { items })
                }

                fn encode(&self, w: &mut Writer) {
                    self.items.encode(w);
                }
            }
        )*
    };
}

move_items_payload! {
    /// 403: reply is 1 Status.
    CreateAlias = CREATE_ALIAS;
    /// 407: reply is 1 Status.
    MoveFileFolder = MOVE_FILE;
}

/// 405: reply is 1 Status.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MoveToTrash {
    /// The folder that holds the items.
    pub folder: FilePath,
    /// The items' names, as a name list.
    pub names: FilePath,
}

impl Payload for MoveToTrash {
    const KIND: MessageType = MessageType::MOVE_TO_TRASH;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = MoveToTrash {
            folder: FilePath::decode_delimited(&mut r)?,
            names: FilePath::decode_delimited(&mut r)?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        self.folder.encode_delimited(w);
        self.names.encode_delimited(w);
    }
}

/// 410: reply to 409 for a file or folder.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileInfo {
    pub ignored_00: u32,
    pub ignored_04: u32,
    pub size: u64,
    pub created: Time1900,
    pub modified: Time1900,
    pub ignored_20: [u8; 8],
    pub ignored_28: u32,
    pub ignored_2c: [u8; 17],
    pub ignored_3f: u8,
    pub name: Vec<u8>,
    /// `folder`, or a type such as `exttype/bin`.
    pub kind: Vec<u8>,
    pub rest: Vec<u8>,
}

impl Payload for FileInfo {
    const KIND: MessageType = MessageType::FILE_INFO;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let ignored_00 = r.u32()?;
        let ignored_04 = r.u32()?;
        let size = r.u64()?;
        let created = Time1900::decode(&mut r)?;
        let modified = Time1900::decode(&mut r)?;
        let ignored_20: [u8; 8] = r.array()?;
        let ignored_28 = r.u32()?;
        let ignored_2c: [u8; 17] = r.array()?;
        let name_len = usize::from(r.u8()?);
        let kind_len = usize::from(r.u8()?);
        let ignored_3f = r.u8()?;
        Ok(FileInfo {
            ignored_00,
            ignored_04,
            size,
            created,
            modified,
            ignored_20,
            ignored_28,
            ignored_2c,
            ignored_3f,
            name: r.bytes(name_len)?,
            kind: r.bytes(kind_len)?,
            rest: r.rest(),
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.ignored_00);
        w.u32(self.ignored_04);
        w.u64(self.size);
        self.created.encode(w);
        self.modified.encode(w);
        w.bytes(&self.ignored_20);
        w.u32(self.ignored_28);
        w.bytes(&self.ignored_2c);
        w.u8(self.name.len() as u8);
        w.u8(self.kind.len() as u8);
        w.u8(self.ignored_3f);
        w.bytes(&self.name);
        w.bytes(&self.kind);
        w.bytes(&self.rest);
    }
}

/// 411
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetFileInfo {
    pub ignored_00: [u8; 40],
    /// Bit 0 renames; bits 0x86 set attributes.
    pub flags: u32,
    pub ignored_2c: [u8; 2],
    pub ignored_30: u8,
    pub ignored_32: [u8; 2],
    pub path: FilePath,
    pub name: Vec<u8>,
}

impl Payload for SetFileInfo {
    const KIND: MessageType = MessageType::SET_FILE_INFO;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let ignored_00: [u8; 40] = r.array()?;
        let flags = r.u32()?;
        let ignored_2c: [u8; 2] = r.array()?;
        let path_len = usize::from(r.u16()?);
        let ignored_30 = r.u8()?;
        let name_len = usize::from(r.u8()?);
        let ignored_32: [u8; 2] = r.array()?;
        let out = SetFileInfo {
            ignored_00,
            flags,
            ignored_2c,
            ignored_30,
            ignored_32,
            path: sized_path(&mut r, path_len)?,
            name: r.bytes(name_len)?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        let path = self.path.to_bytes();
        w.bytes(&self.ignored_00);
        w.u32(self.flags);
        w.bytes(&self.ignored_2c);
        w.u16(path.len() as u16);
        w.u8(self.ignored_30);
        w.u8(self.name.len() as u8);
        w.bytes(&self.ignored_32);
        w.bytes(&path);
        w.bytes(&self.name);
    }
}

/// 412: on a transfer connection; reply is 413, then one 414 per fork.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DownloadFile {
    /// Where to resume; rounded down to a multiple of 0x1000 by the server.
    pub data_offset: u64,
    pub rsrc_offset: u64,
    pub ignored: [u8; 7],
    /// Bit 0: plain (unencrypted) transfer.
    pub options: u8,
    pub path: FilePath,
}

impl DownloadFile {
    pub const OPTION_PLAIN: u8 = 1;
}

impl Payload for DownloadFile {
    const KIND: MessageType = MessageType::DOWNLOAD_FILE;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.len() < 0x1a {
            return Err(DecodeError::Invalid(
                "download request shorter than 0x1a bytes",
            ));
        }
        let mut r = Reader::new(bytes);
        let data_offset = r.u64()?;
        let rsrc_offset = r.u64()?;
        let ignored: [u8; 7] = r.array()?;
        let options = r.u8()?;
        let path = FilePath::decode(&r.rest())?;
        Ok(DownloadFile {
            data_offset,
            rsrc_offset,
            ignored,
            options,
            path,
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u64(self.data_offset);
        w.u64(self.rsrc_offset);
        w.bytes(&self.ignored);
        w.u8(self.options);
        w.bytes(&self.path.to_bytes());
    }
}

/// A file's name and three type strings, in 413 and 415.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileStrings {
    pub name: Vec<u8>,
    /// `exttype/bin`: a type made from the extension.
    pub second: Vec<u8>,
    pub third: Vec<u8>,
    /// At most 0x41 bytes.
    pub fourth: Vec<u8>,
}

impl FileStrings {
    fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        let name_len = usize::from(r.u8()?);
        let second_len = usize::from(r.u8()?);
        let third_len = usize::from(r.u8()?);
        let fourth_len = usize::from(r.u8()?);
        Ok(FileStrings {
            name: r.bytes(name_len)?,
            second: r.bytes(second_len)?,
            third: r.bytes(third_len)?,
            fourth: r.bytes(fourth_len)?,
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u8(self.name.len() as u8);
        w.u8(self.second.len() as u8);
        w.u8(self.third.len() as u8);
        w.u8(self.fourth.len() as u8);
        w.bytes(&self.name);
        w.bytes(&self.second);
        w.bytes(&self.third);
        w.bytes(&self.fourth);
    }
}

/// 413: reply to 412.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DownloadInfo {
    pub data_size: u64,
    pub rsrc_size: u64,
    pub ignored_10: [u8; 8],
    pub created: Time1900,
    pub modified: Time1900,
    pub ignored_28: [u8; 8],
    pub ignored_30: u32,
    pub ignored_34: [u8; 20],
    pub strings: FileStrings,
}

impl Payload for DownloadInfo {
    const KIND: MessageType = MessageType::DOWNLOAD_FILE_REPLY;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = DownloadInfo {
            data_size: r.u64()?,
            rsrc_size: r.u64()?,
            ignored_10: r.array()?,
            created: Time1900::decode(&mut r)?,
            modified: Time1900::decode(&mut r)?,
            ignored_28: r.array()?,
            ignored_30: r.u32()?,
            ignored_34: r.array()?,
            strings: FileStrings::decode(&mut r)?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u64(self.data_size);
        w.u64(self.rsrc_size);
        w.bytes(&self.ignored_10);
        self.created.encode(w);
        self.modified.encode(w);
        w.bytes(&self.ignored_28);
        w.u32(self.ignored_30);
        w.bytes(&self.ignored_34);
        self.strings.encode(w);
    }
}

/// 414 and 417: the bytes of one fork, with no framing. One header covers
/// the whole fork; codecs stream it rather than decode it whole.
macro_rules! file_data_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq, Default)]
            pub struct $name {
                pub data: Vec<u8>,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    Ok($name { data: bytes.to_vec() })
                }

                fn encode(&self, w: &mut Writer) {
                    w.bytes(&self.data);
                }
            }
        )*
    };
}

file_data_payload! {
    /// 414
    DownloadFileData = DOWNLOAD_FILE_DATA;
    /// 417
    UploadFileData = UPLOAD_FILE_DATA;
}

/// 415: on a transfer connection; reply is 416, then the client sends one
/// 417 per fork and the server ends with 418.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UploadFile {
    pub data_size: u64,
    pub rsrc_size: u64,
    pub ignored_10: [u8; 7],
    /// Bit 0: plain (unencrypted) transfer.
    pub options: u8,
    /// Probably the times, as in 413.
    pub ignored_18: [u8; 24],
    pub ignored_30: [u8; 22],
    /// The name must not be empty.
    pub strings: FileStrings,
    /// The destination folder.
    pub folder: FilePath,
    pub rest: Vec<u8>,
}

impl Payload for UploadFile {
    const KIND: MessageType = MessageType::UPLOAD_FILE;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let data_size = r.u64()?;
        let rsrc_size = r.u64()?;
        let ignored_10: [u8; 7] = r.array()?;
        let options = r.u8()?;
        let ignored_18: [u8; 24] = r.array()?;
        let ignored_30: [u8; 22] = r.array()?;
        let folder_len = usize::from(r.u16()?);
        let strings = FileStrings::decode(&mut r)?;
        let folder = sized_path(&mut r, folder_len)?;
        Ok(UploadFile {
            data_size,
            rsrc_size,
            ignored_10,
            options,
            ignored_18,
            ignored_30,
            strings,
            folder,
            rest: r.rest(),
        })
    }

    fn encode(&self, w: &mut Writer) {
        let folder = self.folder.to_bytes();
        w.u64(self.data_size);
        w.u64(self.rsrc_size);
        w.bytes(&self.ignored_10);
        w.u8(self.options);
        w.bytes(&self.ignored_18);
        w.bytes(&self.ignored_30);
        w.u16(folder.len() as u16);
        self.strings.encode(w);
        w.bytes(&folder);
        w.bytes(&self.rest);
    }
}

/// 416: reply to 415.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UploadReply {
    /// Where to resume, rounded down to a multiple of 0x1000.
    pub data_offset: u64,
    /// All ones: resource forks do not resume.
    pub rsrc_offset: u64,
    pub ignored: [u8; 8],
}

impl Payload for UploadReply {
    const KIND: MessageType = MessageType::UPLOAD_FILE_REPLY;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = UploadReply {
            data_offset: r.u64()?,
            rsrc_offset: r.u64()?,
            ignored: r.array()?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u64(self.data_offset);
        w.u64(self.rsrc_offset);
        w.bytes(&self.ignored);
    }
}

/// 419
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NotifySpeedLimit {
    pub bytes_per_second: u32,
}

impl Payload for NotifySpeedLimit {
    const KIND: MessageType = MessageType::NOTIFY_SPEED_LIMIT;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let bytes_per_second = r.u32()?;
        r.finish()?;
        Ok(NotifySpeedLimit { bytes_per_second })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.bytes_per_second);
    }
}

/// 420: reply is 1 Status 0x1a4.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LaunchFile {
    pub ignored: [u8; 8],
    pub path: FilePath,
}

impl Payload for LaunchFile {
    const KIND: MessageType = MessageType::LAUNCH_FILE;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.len() < 9 {
            return Err(DecodeError::Invalid("launch request shorter than 9 bytes"));
        }
        let mut r = Reader::new(bytes);
        let ignored: [u8; 8] = r.array()?;
        let path = FilePath::decode(&r.rest())?;
        Ok(LaunchFile { ignored, path })
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.ignored);
        w.bytes(&self.path.to_bytes());
    }
}

/// 422: reply to 409 for a volume.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VolumeInfo {
    /// All ones if unknown.
    pub total: u64,
    pub free: u64,
    pub modified: Time1900,
    pub created: Time1900,
    /// 0xffffffff if unknown.
    pub item_count: u32,
    pub ignored_24: [u8; 2],
    /// 1 RAM, 2 Network, 3 Cartridge, 4 Hard Disk, 5 CD, 6 DVD, 7 Floppy; 0 on Linux.
    pub disk_type: u8,
    pub ignored_27: [u8; 5],
    pub ignored_2c: u32,
    pub icon: u32,
    pub ignored_34: [u8; 16],
    /// Bit 0: the name can be edited. Bit 1: the comments can be edited.
    pub flags: u32,
    pub flags_mask: u32,
    pub ignored_4c: [u8; 2],
    pub ignored_4f: u8,
    pub ignored_52: [u8; 2],
    pub name: Vec<u8>,
    /// The file system name, such as `tmpfs`.
    pub format: Vec<u8>,
    pub comments: Vec<u8>,
}

impl Payload for VolumeInfo {
    const KIND: MessageType = MessageType::VOLUME_INFO;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let total = r.u64()?;
        let free = r.u64()?;
        let modified = Time1900::decode(&mut r)?;
        let created = Time1900::decode(&mut r)?;
        let item_count = r.u32()?;
        let ignored_24: [u8; 2] = r.array()?;
        let disk_type = r.u8()?;
        let ignored_27: [u8; 5] = r.array()?;
        let ignored_2c = r.u32()?;
        let icon = r.u32()?;
        let ignored_34: [u8; 16] = r.array()?;
        let flags = r.u32()?;
        let flags_mask = r.u32()?;
        let ignored_4c: [u8; 2] = r.array()?;
        let name_len = usize::from(r.u8()?);
        let ignored_4f = r.u8()?;
        let format_len = usize::from(r.u8()?);
        let comments_len = usize::from(r.u8()?);
        let ignored_52: [u8; 2] = r.array()?;
        let out = VolumeInfo {
            total,
            free,
            modified,
            created,
            item_count,
            ignored_24,
            disk_type,
            ignored_27,
            ignored_2c,
            icon,
            ignored_34,
            flags,
            flags_mask,
            ignored_4c,
            ignored_4f,
            ignored_52,
            name: r.bytes(name_len)?,
            format: r.bytes(format_len)?,
            comments: r.bytes(comments_len)?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u64(self.total);
        w.u64(self.free);
        self.modified.encode(w);
        self.created.encode(w);
        w.u32(self.item_count);
        w.bytes(&self.ignored_24);
        w.u8(self.disk_type);
        w.bytes(&self.ignored_27);
        w.u32(self.ignored_2c);
        w.u32(self.icon);
        w.bytes(&self.ignored_34);
        w.u32(self.flags);
        w.u32(self.flags_mask);
        w.bytes(&self.ignored_4c);
        w.u8(self.name.len() as u8);
        w.u8(self.ignored_4f);
        w.u8(self.format.len() as u8);
        w.u8(self.comments.len() as u8);
        w.bytes(&self.ignored_52);
        w.bytes(&self.name);
        w.bytes(&self.format);
        w.bytes(&self.comments);
    }
}

/// 423
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetVolumeInfo {
    pub ignored_00: [u8; 68],
    /// Bit 0 renames the volume. Bit 1 sets the comments.
    pub flags: u32,
    pub ignored_48: [u8; 6],
    pub ignored_52: [u8; 2],
    /// The volume's current name.
    pub name: Vec<u8>,
    pub new_name: Vec<u8>,
    pub format: Vec<u8>,
    pub comments: Vec<u8>,
}

impl Payload for SetVolumeInfo {
    const KIND: MessageType = MessageType::SET_VOLUME_INFO;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let ignored_00: [u8; 68] = r.array()?;
        let flags = r.u32()?;
        let ignored_48: [u8; 6] = r.array()?;
        let name_len = usize::from(r.u8()?);
        let new_name_len = usize::from(r.u8()?);
        let format_len = usize::from(r.u8()?);
        let comments_len = usize::from(r.u8()?);
        let ignored_52: [u8; 2] = r.array()?;
        let out = SetVolumeInfo {
            ignored_00,
            flags,
            ignored_48,
            ignored_52,
            name: r.bytes(name_len)?,
            new_name: r.bytes(new_name_len)?,
            format: r.bytes(format_len)?,
            comments: r.bytes(comments_len)?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.ignored_00);
        w.u32(self.flags);
        w.bytes(&self.ignored_48);
        w.u8(self.name.len() as u8);
        w.u8(self.new_name.len() as u8);
        w.u8(self.format.len() as u8);
        w.u8(self.comments.len() as u8);
        w.bytes(&self.ignored_52);
        w.bytes(&self.name);
        w.bytes(&self.new_name);
        w.bytes(&self.format);
        w.bytes(&self.comments);
    }
}

/// 450: progress comes as 451.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GenerateCatalog {
    /// Bit 0 cancels. `None` when the client sends no payload (0).
    pub flags: Option<u32>,
}

impl GenerateCatalog {
    pub const FLAG_CANCEL: u32 = 1;
}

impl Payload for GenerateCatalog {
    const KIND: MessageType = MessageType::GENERATE_CATALOG;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.is_empty() {
            return Ok(GenerateCatalog { flags: None });
        }
        let mut r = Reader::new(bytes);
        let flags = r.u32()?;
        r.finish()?;
        Ok(GenerateCatalog { flags: Some(flags) })
    }

    fn encode(&self, w: &mut Writer) {
        if let Some(f) = self.flags {
            w.u32(f);
        }
    }
}

/// 451
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GenerateCatalogStatus {
    /// 1: done.
    pub flags: u32,
    /// Items indexed so far.
    pub items: u32,
}

impl Payload for GenerateCatalogStatus {
    const KIND: MessageType = MessageType::GENERATE_CATALOG_STATUS;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = GenerateCatalogStatus {
            flags: r.u32()?,
            items: r.u32()?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.flags);
        w.u32(self.items);
    }
}

/// 452: reply is 453, or 1 Status 0x640001 with no catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSearch {
    pub ignored_00: [u8; 84],
    pub ignored_55: [u8; 3],
    /// At least 3 bytes.
    pub query: Vec<u8>,
}

impl Payload for FileSearch {
    const KIND: MessageType = MessageType::FILE_SEARCH;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let ignored_00: [u8; 84] = r.array()?;
        let query_len = usize::from(r.u8()?);
        let ignored_55: [u8; 3] = r.array()?;
        let query = r.bytes(query_len)?;
        r.finish()?;
        Ok(FileSearch {
            ignored_00,
            ignored_55,
            query,
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.ignored_00);
        w.u8(self.query.len() as u8);
        w.bytes(&self.ignored_55);
        w.bytes(&self.query);
    }
}

/// One match in 453.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileSearchEntry {
    pub size: u64,
    pub created: Time1900,
    pub modified: Time1900,
    pub ignored_1c: [u8; 4],
    pub attributes: u32,
    pub ignored_24: [u8; 10],
    pub ignored_2e: u16,
    pub ignored_30: [u8; 4],
    pub ignored_34: u8,
    pub ignored_35: u8,
    pub ignored_37: u8,
    pub name: Vec<u8>,
    /// The item's full path, root first.
    pub path: FilePath,
    pub padding: Vec<u8>,
}

/// 453: reply to 452; at most 100 entries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileSearchReply {
    pub entries: Vec<FileSearchEntry>,
}

impl Payload for FileSearchReply {
    const KIND: MessageType = MessageType::FILE_SEARCH_REPLY;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let count = r.u32()? as usize;
        let mut entries = Vec::with_capacity(count.min(100));
        for _ in 0..count {
            let start = r.pos();
            let size = r.u64()?;
            let created = Time1900::decode(&mut r)?;
            let modified = Time1900::decode(&mut r)?;
            let path_len = r.u32()? as usize;
            let ignored_1c: [u8; 4] = r.array()?;
            let attributes = r.u32()?;
            let ignored_24: [u8; 10] = r.array()?;
            let ignored_2e = r.u16()?;
            let ignored_30: [u8; 4] = r.array()?;
            let ignored_34 = r.u8()?;
            let ignored_35 = r.u8()?;
            let name_len = usize::from(r.u8()?);
            let ignored_37 = r.u8()?;
            entries.push(FileSearchEntry {
                size,
                created,
                modified,
                ignored_1c,
                attributes,
                ignored_24,
                ignored_2e,
                ignored_30,
                ignored_34,
                ignored_35,
                ignored_37,
                name: r.bytes(name_len)?,
                path: sized_path(&mut r, path_len)?,
                padding: r.pad4(start)?,
            });
        }
        r.finish()?;
        Ok(FileSearchReply { entries })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.entries.len() as u32);
        for e in &self.entries {
            let start = w.pos();
            let path = e.path.to_bytes();
            w.u64(e.size);
            e.created.encode(w);
            e.modified.encode(w);
            w.u32(path.len() as u32);
            w.bytes(&e.ignored_1c);
            w.u32(e.attributes);
            w.bytes(&e.ignored_24);
            w.u16(e.ignored_2e);
            w.bytes(&e.ignored_30);
            w.u8(e.ignored_34);
            w.u8(e.ignored_35);
            w.u8(e.name.len() as u8);
            w.u8(e.ignored_37);
            w.bytes(&e.name);
            w.bytes(&path);
            w.pad4(start, &e.padding);
        }
    }
}
