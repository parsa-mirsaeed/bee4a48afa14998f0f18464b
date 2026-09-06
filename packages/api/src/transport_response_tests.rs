//! Exercise the actual fullstack decoder against complete and truncated bodies.

use dioxus::fullstack::{ClientRequest, RequestDecodeResult, ServerFnDecoder, ServerFnError};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn decode_response(body: &'static str, declared_length: usize) -> bool {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local transport fixture");
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut chunk = [0_u8; 1024];
        loop {
            let count = socket.read(&mut chunk).await.unwrap();
            assert!(count > 0, "request headers must complete");
            request.extend_from_slice(&chunk[..count]);
            assert!(request.len() <= 4096, "bounded fixture request");
            if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                break;
            }
        }
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {declared_length}\r\nConnection: close\r\n\r\n{body}"
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
    });
    let request = ClientRequest {
        url: format!("http://{address}/transport-regression")
            .parse()
            .unwrap(),
        method: dioxus::fullstack::http::Method::GET,
        headers: Default::default(),
        extensions: Default::default(),
    };
    let response = tokio::time::timeout(Duration::from_secs(5), request.send_empty_body())
        .await
        .expect("local request completes")
        .expect("headers arrive successfully");
    assert_eq!(response.status().as_u16(), 200);
    let decoder = ServerFnDecoder::<Result<serde_json::Value, ServerFnError>>::new();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        (&&decoder).decode_client_response(Ok(response)),
    )
    .await
    .expect("body read completes");
    server.await.unwrap();
    match result {
        Ok(Ok(value)) => {
            assert_eq!(value, serde_json::json!({ "value": 7 }));
            true
        }
        Ok(Err(error)) => panic!("unexpected decoder error: {error}"),
        Err(_) => false,
    }
}

#[tokio::test]
async fn complete_response_body_still_decodes() {
    let body = r#"{"value":7}"#;
    assert!(decode_response(body, body.len()).await);
}

#[tokio::test]
async fn truncated_response_body_returns_request_error_without_panicking() {
    assert!(!decode_response("{", 100).await);
}
