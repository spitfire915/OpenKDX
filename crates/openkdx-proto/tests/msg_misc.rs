//! Messages 800–804 (processes.md), 900–905 (view-display.md),
//! 1000–1003 (voice.md), and 1100–1107 (folder-access.md).

mod common;

use common::{rejects_trailing, round_trip};
use openkdx_proto::msg::common::Time1900;
use openkdx_proto::msg::misc::*;

#[test]
fn no_payload_messages() {
    assert_eq!(round_trip(OpenProcessMonitor), Vec::<u8>::new());
    assert_eq!(round_trip(CloseProcessMonitor), Vec::<u8>::new());
    assert_eq!(round_trip(GetProcessList), Vec::<u8>::new());
    assert_eq!(round_trip(OpenViewDisplay), Vec::<u8>::new());
    assert_eq!(round_trip(StartVoice), Vec::<u8>::new());
}

#[test]
fn process_list() {
    // 0x3c + 7 + 0 + 4 = 71 → 1 byte
    let bytes = round_trip(ProcessList {
        memory_used: 1,
        memory_total: 2,
        ignored: [0; 12],
        processes: vec![Process {
            pid: 1,
            pid_high: 0,
            start_time: Time1900(3),
            memory_used: 0,
            memory_size: 0,
            cpu_time: 5,
            background: 0,
            cpu_percent: 0x32,
            memory_percent: 0xff,
            ignored_2a: [0; 15],
            command: b"/bin/ps".to_vec(),
            string_3b: vec![],
            user: b"root".to_vec(),
            padding: vec![0],
        }],
    });
    assert_eq!(bytes.len(), 0x20 + 72);
    assert_eq!(&bytes[0x20 + 0x3c..0x20 + 0x43], b"/bin/ps");
    rejects_trailing::<ProcessList>(&bytes);
    assert_eq!(
        round_trip(ExitTerminateProcess {
            flags: 1,
            pid: 42,
            value_08: 0
        }),
        [0, 0, 0, 1, 0, 0, 0, 42, 0, 0, 0, 0]
    );
}

#[test]
fn view_display() {
    round_trip(CloseViewDisplay { ignored: vec![1] });
    let bytes = round_trip(ViewDisplayParams {
        flags: 1,
        ignored: [0; 12],
    });
    assert_eq!(bytes.len(), 16);
    let bytes = round_trip(ViewDisplayUpdate {
        screen_width: 640,
        screen_height: 480,
        left: 0,
        top: 0,
        right: 2,
        bottom: 1,
        pixels: vec![0xff, 0xff, 0, 0],
    });
    assert_eq!(bytes.len(), 12 + 4);
    let bytes = round_trip(ViewDisplayMouse {
        modifiers: 0,
        x: 10,
        y: 20,
        wheel: 0,
        button: 1,
        kind: 1,
    });
    assert_eq!(bytes.len(), 12);
    let bytes = round_trip(ViewDisplayKeyboard {
        modifiers: 8,
        character: b'A' as u16,
        key_code: 4,
        ignored: [0; 3],
        kind: 1,
    });
    assert_eq!(bytes.len(), 12);
}

#[test]
fn voice() {
    assert_eq!(round_trip(StartVoiceReply { result: 0 }), [0; 4]);
    let bytes = round_trip(VoiceFormat {
        ignored: [0; 12],
        sample_rate: 8000,
        codec: *b"ADPC",
        buffer: 100,
        channels: 1,
        bits_per_sample: 16,
    });
    assert_eq!(bytes.len(), 24);
    assert_eq!(&bytes[0x10..0x14], b"ADPC");
    assert_eq!(
        round_trip(VoiceData {
            audio: vec![1, 2, 3]
        }),
        [1, 2, 3]
    );
}

fn folder_access(current: &[u8], pad: &[u8]) -> FolderAccess {
    let mut rules = [FolderAccessRule::default(); 32];
    rules[0] = FolderAccessRule {
        permission_bits: 0xfffe,
        ignored: 0,
    };
    FolderAccess {
        ignored_00: [0; 4],
        flags: 2,
        icon: 0x2c,
        class_rules: rules,
        rule: rules[0],
        ignored_90: [0; 29],
        ignored_b0: 0,
        ignored_b2: [0; 6],
        name: b"DB".to_vec(),
        owner: b"admin".to_vec(),
        description: b"drop".to_vec(),
        current_name: current.to_vec(),
        padding: pad.to_vec(),
    }
}

#[test]
fn folder_access_items() {
    // 0xb8 + 2 + 5 + 4 = 195 → 1 byte
    let bytes = round_trip(CreateFolderAccess {
        item: folder_access(b"", &[0]),
    });
    assert_eq!(bytes.len(), 196);
    assert_eq!(&bytes[0x0c..0x10], &[0xff, 0xfe, 0, 0]);
    assert!(bytes[0x0c..0x10].iter().any(|&b| b != 0));
    rejects_trailing::<CreateFolderAccess>(&bytes);
    round_trip(FolderAccessInfo {
        item: folder_access(b"", &[0]),
    });
    // 195 + 2 = 197 → 3 bytes
    round_trip(SetFolderAccess {
        item: folder_access(b"DB", &[0, 0, 0]),
    });
    assert_eq!(
        round_trip(DeleteFolderAccess {
            name: b"DB".to_vec()
        }),
        b"DB"
    );
    assert_eq!(
        round_trip(FolderAccessDeleted {
            name: b"DB".to_vec()
        }),
        b"DB"
    );
    assert_eq!(
        round_trip(GetFolderAccess {
            name: b"DB".to_vec()
        }),
        b"DB"
    );
    assert_eq!(
        round_trip(GetFolderAccessList { rest: vec![] }),
        Vec::<u8>::new()
    );
    let g = GetFolderAccessList {
        rest: vec![0, 0, 0, 1, 9],
    };
    assert_eq!(g.flags(), Some(1));
    round_trip(g);
    // 0x14 + 9 + 0 + 3 = 32 → none
    let bytes = round_trip(FolderAccessList {
        items: vec![FolderAccessItem {
            flags: 1,
            icon: 0,
            ignored_08: [0; 8],
            ignored_10: 0,
            name: b"[Default]".to_vec(),
            second: vec![],
            description: b"If ".to_vec(),
            padding: vec![],
        }],
    });
    assert_eq!(bytes.len(), 4 + 32);
}
