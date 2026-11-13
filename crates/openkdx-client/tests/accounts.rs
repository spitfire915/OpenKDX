mod common;

use common::fixture::session_with;
use common::mock::Step;
use openkdx_client::accounts::{format_privileges, new_account, parse_privileges};
use openkdx_proto::AccountKey;
use openkdx_proto::msg::Status;
use openkdx_proto::msg::accounts::{
    AccountInfo, AccountList, AccountListEntry, Privilege, Privileges, SetAccount,
};
use openkdx_proto::msg::misc::{FolderAccessItem, FolderAccessList, GetFolderAccessList};
use openkdx_proto::{Message, MessageType, Payload};

#[test]
fn parse_privileges_round_trips() {
    let p = parse_privileges("can-login,download,upload").unwrap();
    assert!(p.has(Privilege::CanLogin));
    assert!(p.has(Privilege::Download));
    assert!(!p.has(Privilege::Broadcast));
    assert_eq!(format_privileges(&p), "can-login,download,upload");
    assert_eq!(format_privileges(&Privileges::NONE), "none");
    assert_eq!(parse_privileges("none").unwrap(), Privileges::NONE);
    assert_eq!(parse_privileges("all").unwrap(), !Privileges::NONE);
    assert!(parse_privileges("fly").is_err());
    assert_eq!(
        parse_privileges(" Can-Login , UPLOAD ").unwrap(),
        parse_privileges("can-login,upload").unwrap()
    );
}

#[test]
fn new_account_sets_key_from_password() {
    let a = new_account(
        b"tester",
        b"Tester",
        Some(b"t"),
        1,
        Privileges::NONE,
        Privileges::NONE,
    );
    assert_eq!(a.key, AccountKey::from_password(b"t").0);
    assert_eq!(a.class_index, 1);
    assert_eq!(a.login, b"tester");
    let keep = new_account(
        b"tester",
        b"Tester",
        None,
        1,
        Privileges::NONE,
        Privileges::NONE,
    );
    assert_eq!(keep.key, [0; 32]);
    assert_ne!(a.created.0, 0);
}

#[tokio::test]
async fn account_list_show_set_and_delete() {
    let shown = AccountInfo {
        account: new_account(
            b"tester",
            b"Tester",
            Some(b"t"),
            1,
            Privileges::NONE,
            Privileges::NONE,
        ),
    };
    let (server, session) = session_with(vec![
        Step::Expect(MessageType::GET_ACCOUNT_LIST),
        Step::Reply(Message::AccountList(AccountList {
            accounts: vec![AccountListEntry {
                class_index: 1,
                login: b"tester".to_vec(),
                name: b"Tester".to_vec(),
                padding: vec![0; 2],
                ..AccountListEntry::default()
            }],
        })),
        Step::Expect(MessageType::GET_ACCOUNT),
        Step::Reply(Message::AccountInfo(shown.clone())),
        Step::Expect(MessageType::SET_ACCOUNT),
        Step::Reply(Message::Status(Status {
            code: 0,
            request_type_or_value: 0x1ff,
            detail: None,
        })),
        Step::Expect(MessageType::DELETE_ACCOUNT),
        Step::Send(Message::AccountDeleted(
            openkdx_proto::msg::accounts::AccountDeleted {
                login: b"tester".to_vec(),
            },
        )),
    ])
    .await;
    let list = session.accounts().await.unwrap();
    assert_eq!(list[0].login, b"tester");
    let acct = session.account(b"tester").await.unwrap();
    assert_eq!(acct.name, b"Tester");
    let mut changed = acct.clone();
    changed.name = b"Tester 2".to_vec();
    session.set_account(changed.clone()).await.unwrap();
    let sent = SetAccount::decode(&server.payloads(MessageType::SET_ACCOUNT)[0]).unwrap();
    assert_eq!(sent.account.name, b"Tester 2");
    session.delete_account(b"tester").await.unwrap();
    assert_eq!(server.payloads(MessageType::DELETE_ACCOUNT)[0], b"tester");
    server.check();
}

#[tokio::test]
async fn folder_access_list_sends_flags_zero() {
    let (server, session) = session_with(vec![
        Step::Expect(MessageType::GET_FOLDER_ACCESS_LIST),
        Step::Reply(Message::FolderAccessList(FolderAccessList {
            items: vec![FolderAccessItem {
                icon: 0x2b,
                name: b"Uploads".to_vec(),
                padding: vec![0; 2],
                ..FolderAccessItem::default()
            }],
        })),
    ])
    .await;
    let items = session.folder_access_list().await.unwrap();
    assert_eq!(items[0].name, b"Uploads");
    let sent =
        GetFolderAccessList::decode(&server.payloads(MessageType::GET_FOLDER_ACCESS_LIST)[0])
            .unwrap();
    assert_eq!(sent.flags(), Some(0));
    server.check();
}
