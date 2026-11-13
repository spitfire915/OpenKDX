mod common;

use std::time::Duration;

use common::mock::{MockServer, key, table};
use openkdx_client::ClientError;
use openkdx_client::handshake::client_handshake;
use openkdx_client::txid::TxidSource;
use openkdx_proto::AccountKey;
use openkdx_proto::handshake::Role;
use tokio::net::TcpStream;

#[tokio::test]
async fn handshake_succeeds_against_mock() {
    let server = MockServer::start(vec![(b"admin".to_vec(), key("pw"))], vec![vec![]]).await;
    let mut stream = TcpStream::connect(server.addr).await.unwrap();
    let out = client_handshake(&mut stream, b"admin", &key("pw"), &table())
        .await
        .unwrap();
    assert_eq!(out.server_hello.role(), Some(Role::Server));
    assert!(out.read_buf.is_empty());
}

#[tokio::test]
async fn handshake_wrong_password_is_login_failed() {
    let server = MockServer::start(vec![(b"admin".to_vec(), key("pw"))], vec![vec![]]).await;
    let mut stream = TcpStream::connect(server.addr).await.unwrap();
    let err = tokio::time::timeout(
        Duration::from_secs(5),
        client_handshake(&mut stream, b"admin", &key("wrong"), &table()),
    )
    .await
    .expect("no hang")
    .unwrap_err();
    assert!(
        matches!(
            err,
            ClientError::LoginFailed(_) | ClientError::ProofMismatch
        ),
        "{err:?}"
    );
}

#[tokio::test]
async fn handshake_rejects_long_login() {
    let (mut a, _b) = tokio::io::duplex(64);
    let err = client_handshake(&mut a, &[b'x'; 32], &AccountKey::GUEST, &table())
        .await
        .unwrap_err();
    assert!(matches!(err, ClientError::LoginTooLong));
}

#[test]
fn txid_source_increments_and_skips_zero() {
    let t = TxidSource::new(u32::MAX - 1);
    assert_eq!(t.next(), u32::MAX - 1);
    assert_eq!(t.next(), u32::MAX);
    assert_eq!(t.next(), 1);
    assert_ne!(TxidSource::new_random().next(), 0);
}
