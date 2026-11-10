//! Messages 502–551 (accounts.md) and the privilege bits (privileges.md).

mod common;

use common::{rejects_trailing, round_trip};
use openkdx_proto::msg::accounts::*;
use openkdx_proto::msg::common::{Color, Time1900};

#[test]
fn privilege_bits_match_spec() {
    // bit n is in byte n/8 with mask 0x80 >> (n % 8)
    let mut p = Privileges::NONE;
    p.set(Privilege::Override, true);
    assert_eq!(p.bytes()[0], 0x80);
    p.set(Privilege::CanLogin, true);
    assert_eq!(p.bytes()[0], 0x88);
    p.set(Privilege::ModifyServerSettings, true);
    assert_eq!(p.bytes()[7], 0x04, "bit 61 is byte 7 mask 0x04");
    p.set(Privilege::Download, true);
    assert_eq!(p.bytes()[1], 0x20, "bit 10 is byte 1 mask 0x20");
    p.set(Privilege::LaunchPrograms, true);
    assert_eq!(p.bytes()[10], 0x20, "bit 82 is byte 10 mask 0x20");
    assert!(p.has(Privilege::Download));
    assert!(!p.has(Privilege::Upload));
    p.set(Privilege::Download, false);
    assert!(!p.has(Privilege::Download));

    // the admin privileges as the spec quotes them: `80 00 00 00 00 00 02 00…`
    let admin = Privileges::from_bytes([0x80, 0, 0, 0, 0, 0, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert!(admin.has(Privilege::Override));
    assert!(
        admin.has(Privilege::ProtectedAccount),
        "bit 54 is byte 6 mask 0x02"
    );
    assert_eq!(Privilege::ProtectedAccount.bit(), 54);
    assert_eq!(Privilege::ProtectedAccount.name(), "Protected Account");
    assert_eq!(Privilege::ALL.len(), 55);
    assert_eq!(
        admin.set_bits(),
        vec![Privilege::Override, Privilege::ProtectedAccount]
    );
    assert_eq!(
        (admin & Privileges::from_bits(&[Privilege::Override])).set_bits(),
        vec![Privilege::Override]
    );
    assert_eq!(
        (admin | Privileges::from_bits(&[Privilege::Upload]))
            .set_bits()
            .len(),
        3
    );
    assert!(!(!admin).has(Privilege::Override));
}

fn class(name: &[u8], pad: &[u8]) -> AccountClassInfo {
    AccountClassInfo {
        privileges: Privileges::from_bits(&[Privilege::CanLogin]),
        time: Time1900(1),
        ignored_18: [0; 13],
        flood_protection: 0x50,
        speed_limit: 0,
        ignored_28: [0; 8],
        class_index: 0,
        color: Color {
            r: 0xff,
            g: 0,
            b: 0,
        },
        ignored_37: 0,
        name: name.to_vec(),
        second_string: vec![],
        editor_login: vec![],
        padding: pad.to_vec(),
    }
}

#[test]
fn account_class_messages() {
    assert_eq!(round_trip(GetAccountClass { class_index: 2 }), [2]);
    assert_eq!(round_trip(GetAccountClassList), Vec::<u8>::new());
    // 0x38 + 13 = 69 bytes for Administrator, as the spec reports, with 3 bytes of padding
    let bytes = round_trip(class(b"Administrator", &[0, 0, 0]));
    assert_eq!(bytes.len(), 72);
    let bytes = round_trip(SetAccountClass {
        class: class(b"Administrator", &[]),
    });
    assert_eq!(bytes.len(), 69);
    rejects_trailing::<SetAccountClass>(&bytes);
    // 0x30 + 13 = 61 → 3 bytes; 0x30 + 4 = 52 → none
    let list = AccountClassList {
        classes: vec![
            AccountClassListEntry {
                privileges: Privileges::NONE,
                time: Time1900(0),
                ignored_18: [0; 13],
                flood_protection: 0,
                speed_limit: 0,
                class_index: 0,
                color: Color {
                    r: 0xff,
                    g: 0,
                    b: 0,
                },
                ignored_2d: [0; 2],
                ignored_2f: 0,
                name: b"Administrator".to_vec(),
                padding: vec![0, 0, 0],
            },
            AccountClassListEntry {
                privileges: Privileges::NONE,
                time: Time1900(0),
                ignored_18: [0; 13],
                flood_protection: 0,
                speed_limit: 0,
                class_index: 1,
                color: Color {
                    r: 0,
                    g: 0,
                    b: 0xff,
                },
                ignored_2d: [0; 2],
                ignored_2f: 0,
                name: b"User".to_vec(),
                padding: vec![],
            },
        ],
    };
    let bytes = round_trip(list);
    assert_eq!(bytes.len(), 4 + 64 + 52);
}

fn account(login: &[u8], pad: &[u8]) -> Account {
    Account {
        privileges: Privileges::NONE,
        privilege_mask: Privileges::NONE,
        key: [0; 32],
        created: Time1900(1),
        modified: Time1900(2),
        last_login: Time1900(0),
        ignored_58: [0; 30],
        class_index: 2,
        color: Color::default(),
        ignored_7e: [0; 2],
        login: login.to_vec(),
        name: b"Lab User".to_vec(),
        third: vec![],
        editor: b"admin".to_vec(),
        padding: pad.to_vec(),
    }
}

#[test]
fn account_messages() {
    // 0x80 + 7 + 8 + 0 + 5 = 148 → no pad; login "lab" → 144 → none; "labu" → 145 → 3
    let bytes = round_trip(CreateAccount {
        account: account(b"labuser", &[]),
    });
    assert_eq!(bytes.len(), 148);
    assert_eq!(bytes[0x7a], 7);
    rejects_trailing::<CreateAccount>(&bytes);
    round_trip(AccountInfo {
        account: account(b"labu", &[0, 0, 0]),
    });
    round_trip(SetAccount {
        account: Account {
            key: [1; 32],
            ..account(b"labuser", &[])
        },
    });
    assert!(!account(b"x", &[]).has_password());

    assert_eq!(
        round_trip(DeleteAccount {
            login: b"guest".to_vec()
        }),
        b"guest"
    );
    assert_eq!(
        round_trip(GetAccount {
            login: b"guest".to_vec()
        }),
        b"guest"
    );
    assert_eq!(
        round_trip(AccountDeleted {
            login: b"guest".to_vec()
        }),
        b"guest"
    );
    assert_eq!(round_trip(GetAccountList), Vec::<u8>::new());
    round_trip(AccountChanged {
        ignored: vec![1, 2],
    });

    // 0x28 + 5 + 5 = 50 → 2 bytes
    let bytes = round_trip(AccountList {
        accounts: vec![AccountListEntry {
            created: Time1900(1),
            last_login: Time1900(0),
            ignored_10: [0; 11],
            ignored_1b: 1,
            ignored_1c: [0; 4],
            class_index: 0,
            ignored_21: 0xff,
            ignored_22: [0; 2],
            ignored_26: [0; 2],
            login: b"admin".to_vec(),
            name: b"Admin".to_vec(),
            padding: vec![0, 0],
        }],
    });
    assert_eq!(bytes.len(), 4 + 52);
}

#[test]
fn passwords() {
    let bytes = round_trip(ChangeOwnPassword {
        account_key: *openkdx_proto::AccountKey::from_password(b"new").as_bytes(),
    });
    assert_eq!(bytes.len(), 32);
    assert_eq!(round_trip(ChangeOwnPasswordDone), Vec::<u8>::new());
}
