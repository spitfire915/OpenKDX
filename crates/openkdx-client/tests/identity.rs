use openkdx_client::{ClientError, Identity, load_kdx_table_from};
use openkdx_proto::msg::common::Color;
use openkdx_proto::msg::session::SetIdentity;
use openkdx_proto::{Message, MessageType, Payload};
use openkdx_table::Cache;

#[test]
fn default_identity_matches_client_1600_shape() {
    let id = Identity::default();
    let user = id.user_record();
    assert_eq!(user.user_id, 0);
    assert_eq!(user.client_port, 10600);
    assert_eq!(user.nick, b"openkdx");
    assert_eq!(
        user.background_color,
        Color {
            r: 0xff,
            g: 0xcc,
            b: 0x00
        }
    );
    assert!(id.registered);

    let bytes = SetIdentity { user }.to_bytes();
    match Message::decode(MessageType::SET_IDENTITY, &bytes).unwrap() {
        Message::SetIdentity(m) => assert_eq!(m.user.nick, b"openkdx"),
        other => panic!("decoded {other:?}"),
    }
}

#[test]
fn no_table_error_names_the_fix() {
    let dir = tempfile::tempdir().unwrap();
    let err = load_kdx_table_from(&Cache::new(dir.path().to_path_buf())).unwrap_err();
    assert!(matches!(err, ClientError::NoTable));
    assert!(err.to_string().contains("openkdx-table fetch"));
}
