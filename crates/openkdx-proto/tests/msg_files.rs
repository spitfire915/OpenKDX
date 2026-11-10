//! Messages 400–453 (files.md).

mod common;

use common::{rejects_trailing, round_trip};
use openkdx_proto::msg::Payload;
use openkdx_proto::msg::common::{FilePath, PathName, Time1900};
use openkdx_proto::msg::files::*;

fn path(names: &[&[u8]]) -> FilePath {
    FilePath {
        names: names.iter().map(|n| PathName::new(n)).collect(),
    }
}

fn entry(name: &[u8], kind: &[u8], folder: bool, pad: &[u8]) -> FileListEntry {
    FileListEntry {
        size: if folder { 0x1000 } else { 3 },
        created: Time1900(1),
        modified: Time1900(2),
        ignored_18: [0; 8],
        attributes: if folder { 0x0100_0002 } else { 1 },
        ignored_24: [0; 10],
        ignored_2e: if folder { 0xffff } else { 0 },
        access_item: 0,
        ignored_32: [0; 2],
        ignored_34: 0,
        ignored_37: 0,
        name: name.to_vec(),
        kind: kind.to_vec(),
        padding: pad.to_vec(),
    }
}

fn strings(name: &[u8]) -> FileStrings {
    FileStrings {
        name: name.to_vec(),
        second: b"exttype/txt".to_vec(),
        third: vec![],
        fourth: vec![],
    }
}

#[test]
fn path_payloads() {
    assert_eq!(
        round_trip(GetFileList {
            path: FilePath::root()
        }),
        Vec::<u8>::new()
    );
    let bytes = round_trip(GetFileList {
        path: path(&[b"Uploads [UL]"]),
    });
    assert_eq!(bytes.len(), 2 + 3 + 12);
    round_trip(GetFileFolderInfo {
        path: path(&[b"a", b"b"]),
    });
    let bytes = round_trip(LaunchFile {
        ignored: [0; 8],
        path: path(&[b"run.sh"]),
    });
    assert_eq!(bytes.len(), 8 + 2 + 3 + 6);
    assert!(LaunchFile::decode(&[0; 8]).is_err(), "at least 9 bytes");
    assert_eq!(round_trip(EmptyTrash), Vec::<u8>::new());
    assert_eq!(round_trip(UploadFileComplete), Vec::<u8>::new());
}

#[test]
fn file_list_entries_keep_uninitialized_padding() {
    // 0x38 + 8 + 0 = 64 → no pad; 0x38 + 8 + 11 = 75 → 1 byte; 0x38 + 7 + 0 = 63 → 1 byte
    let list = FileList {
        entries: vec![
            entry(b"Admin Dr", b"", true, &[]),
            entry(b"tiny.txt", b"exttype/txt", false, &[0x86]),
            entry(b"Uploads", b"", true, &[0xab]),
        ],
    };
    let bytes = round_trip(list);
    assert_eq!(bytes.len(), 4 + 64 + 76 + 64);
    assert_eq!(bytes[4 + 64 + 75], 0x86);
    assert_eq!(
        FileList::decode(&[]).unwrap().entries.len(),
        0,
        "an empty folder"
    );
    assert_eq!(FileList { entries: vec![] }.to_bytes(), Vec::<u8>::new());
}

#[test]
fn create_folder_and_moves() {
    let bytes = round_trip(CreateFolder {
        ignored: [0; 19],
        name: b"lab-folder".to_vec(),
        access: b"DB".to_vec(),
        owner: b"admin".to_vec(),
        path: path(&[b"Uploads [UL]"]),
    });
    assert_eq!(bytes[0x13], 2);
    assert_eq!(&bytes[0x14..0x16], &[0, 17]);
    assert_eq!(bytes[0x16], 10);
    assert_eq!(bytes[0x17], 5);
    rejects_trailing::<CreateFolder>(&bytes);
    // an empty path is the root
    round_trip(CreateFolder {
        ignored: [0; 19],
        name: b"x".to_vec(),
        access: vec![],
        owner: vec![],
        path: FilePath::root(),
    });

    let items = MoveItems {
        destination: FilePath::root(),
        source: path(&[b"Uploads [UL]"]),
        names: path(&[b"a.txt", b"b.txt"]),
    };
    let bytes = round_trip(CreateAlias {
        items: items.clone(),
    });
    assert_eq!(&bytes[..2], &[0, 0], "root destination is 00 00");
    round_trip(MoveFileFolder { items });
    let bytes = round_trip(MoveToTrash {
        folder: path(&[b"Uploads [UL]"]),
        names: FilePath::root(),
    });
    assert_eq!(&bytes[bytes.len() - 2..], &[0, 0], "an empty list is 00 00");
}

#[test]
fn unhandled_requests_keep_their_bytes() {
    round_trip(DeleteFileFolder {
        ignored: vec![1, 2, 3],
    });
    round_trip(RenameFileFolder { ignored: vec![] });
    round_trip(CopyFileFolder { ignored: vec![9] });
}

#[test]
fn file_info_and_set_file_info() {
    let bytes = round_trip(FileInfo {
        ignored_00: 0x39,
        ignored_04: 0xbf,
        size: 0x1000,
        created: Time1900(1),
        modified: Time1900(2),
        ignored_20: [0; 8],
        ignored_28: 2,
        ignored_2c: [0; 17],
        ignored_3f: 0,
        name: b"Uploads [UL]".to_vec(),
        kind: b"folder".to_vec(),
        rest: vec![],
    });
    assert_eq!(bytes.len(), 0x40 + 12 + 6); // 82 bytes for Uploads [UL]
    let bytes = round_trip(SetFileInfo {
        ignored_00: [0; 40],
        flags: 1,
        ignored_2c: [0; 2],
        ignored_30: 0,
        ignored_32: [0; 2],
        path: path(&[b"Uploads [UL]", b"upload.txt"]),
        name: b"upload2.txt".to_vec(),
    });
    assert_eq!(&bytes[0x2e..0x30], &[0, 2 + 15 + 13]);
    assert_eq!(bytes[0x31], 11);
}

#[test]
fn download_request_reply_and_data() {
    let bytes = round_trip(DownloadFile {
        data_offset: 0,
        rsrc_offset: 0,
        ignored: [0; 7],
        options: 0,
        path: path(&[b"tiny.txt"]),
    });
    assert_eq!(bytes.len(), 0x18 + 2 + 3 + 8); // 37, as captured
    assert!(
        DownloadFile::decode(&[0; 0x19]).is_err(),
        "at least 0x1a bytes"
    );

    let bytes = round_trip(DownloadInfo {
        data_size: 3,
        rsrc_size: 0,
        ignored_10: [0; 8],
        created: Time1900(1),
        modified: Time1900(2),
        ignored_28: [0; 8],
        ignored_30: 1,
        ignored_34: [0; 20],
        strings: strings(b"tiny.txt"),
    });
    assert_eq!(bytes.len(), 0x48 + 4 + 8 + 11); // 94, as captured
    rejects_trailing::<DownloadInfo>(&bytes);

    assert_eq!(
        round_trip(DownloadFileData {
            data: b"abc".to_vec()
        }),
        b"abc"
    );
    assert_eq!(
        round_trip(UploadFileData { data: vec![] }),
        Vec::<u8>::new()
    );
}

#[test]
fn upload_request_and_reply() {
    let bytes = round_trip(UploadFile {
        data_size: 10,
        rsrc_size: 0,
        ignored_10: [0; 7],
        options: 0,
        ignored_18: [0; 24],
        ignored_30: [0; 22],
        strings: strings(b"up.txt"),
        folder: path(&[b"Uploads [UL]"]),
        rest: vec![],
    });
    assert_eq!(&bytes[0x46..0x48], &[0, 17]);
    assert_eq!(bytes.len(), 0x48 + 4 + 6 + 11 + 17);
    // the folder may be the root
    round_trip(UploadFile {
        data_size: 10,
        rsrc_size: 0,
        ignored_10: [0; 7],
        options: 1,
        ignored_18: [0; 24],
        ignored_30: [0; 22],
        strings: strings(b"up.txt"),
        folder: FilePath::root(),
        rest: vec![7],
    });
    let bytes = round_trip(UploadReply {
        data_offset: 0x1000,
        rsrc_offset: u64::MAX,
        ignored: [0; 8],
    });
    assert_eq!(bytes.len(), 24);
    assert_eq!(
        round_trip(NotifySpeedLimit {
            bytes_per_second: 50_000
        })
        .len(),
        4
    );
}

#[test]
fn volumes() {
    let bytes = round_trip(VolumeInfo {
        total: u64::MAX,
        free: 5,
        modified: Time1900(1),
        created: Time1900(2),
        item_count: 0xffff_ffff,
        ignored_24: [0; 2],
        disk_type: 0,
        ignored_27: [0; 5],
        ignored_2c: 0x88,
        icon: 0x3d,
        ignored_34: [0; 16],
        flags: 0,
        flags_mask: 0x44,
        ignored_4c: [0; 2],
        ignored_4f: 0,
        ignored_52: [0; 2],
        name: b"/tmp".to_vec(),
        format: b"tmpfs".to_vec(),
        comments: vec![],
    });
    assert_eq!(bytes.len(), 0x54 + 4 + 5); // 93, as in the reference capture
    round_trip(SetVolumeInfo {
        ignored_00: [0; 68],
        flags: 1,
        ignored_48: [0; 6],
        ignored_52: [0; 2],
        name: b"/tmp".to_vec(),
        new_name: b"Lab".to_vec(),
        format: vec![],
        comments: b"c".to_vec(),
    });
}

#[test]
fn catalog_and_search() {
    assert_eq!(round_trip(GenerateCatalog { flags: Some(1) }), [0, 0, 0, 1]);
    assert_eq!(
        round_trip(GenerateCatalog { flags: None }),
        Vec::<u8>::new()
    );
    assert_eq!(
        round_trip(GenerateCatalogStatus {
            flags: 1,
            items: 42
        }),
        [0, 0, 0, 1, 0, 0, 0, 42]
    );
    let bytes = round_trip(FileSearch {
        ignored_00: [0; 84],
        ignored_55: [0; 3],
        query: b"tiny".to_vec(),
    });
    assert_eq!(bytes.len(), 0x58 + 4);
    // 0x38 + 4 + (2 + 3 + 4) = 69 → 3 bytes of padding
    let list = FileSearchReply {
        entries: vec![FileSearchEntry {
            size: 3,
            created: Time1900(1),
            modified: Time1900(2),
            ignored_1c: [0; 4],
            attributes: 1,
            ignored_24: [0; 10],
            ignored_2e: 0,
            ignored_30: [0; 4],
            ignored_34: 0x10,
            ignored_35: 0,
            ignored_37: 0,
            name: b"tiny".to_vec(),
            path: path(&[b"tiny"]),
            padding: vec![0, 0, 0],
        }],
    };
    let bytes = round_trip(list);
    assert_eq!(bytes.len(), 4 + 72);
    assert_eq!(&bytes[4 + 0x18..4 + 0x1c], &[0, 0, 0, 9]);
    assert_eq!(
        round_trip(FileSearchReply { entries: vec![] }),
        [0, 0, 0, 0]
    );
}

#[test]
fn file_name_with_invalid_utf8_round_trips() {
    let name = b"Erik\xd5s.txt";
    let bytes = round_trip(GetFileList {
        path: path(&[name]),
    });
    let p = GetFileList::decode(&bytes).unwrap().path;
    assert_eq!(p.names[0].name, name);
    assert_eq!(p.to_string(), "Erik\u{fffd}s.txt");
}
