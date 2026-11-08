use super::*;
use std::io::{Read, Write};

fn completion(content: Value, reason: &str) -> Value {
    json!({"choices":[{"finish_reason":reason,"message":{"content":content.to_string()}}]})
}

#[test]
fn replies_require_completed_strict_proposals_and_never_expose_reasoning() {
    let good = json!({"schemaVersion":1,"narration":"The gate opens onto a quiet road.","choices":["I examine the road."]});
    assert!(parse_completion(completion(good.clone(), "stop")).is_ok());
    assert!(parse_completion(completion(good, "length")).is_err());
    assert!(parse_completion(completion(
        json!({"schemaVersion":1,"narration":"Done","choices":[],"hp":999}),
        "stop"
    ))
    .is_err());
    assert!(parse_completion(completion(
        json!({"schemaVersion":1,"narration":"<think>hidden</think>","choices":[]}),
        "stop"
    ))
    .is_err());
}

#[test]
fn model_read_requests_are_typed_and_write_requests_are_rejected() {
    let request = json!({"schemaVersion":1,"narration":"","choices":[],"requests":[{"kind":"arithmetic","operation":"add","a":2,"b":3}]});
    let decoded = decode_completion(completion(request, "stop")).unwrap();
    assert_eq!(decoded.requests.len(), 1);
    let bad = json!({"schemaVersion":1,"narration":"","choices":[],"requests":[{"kind":"write","path":"campaign.sqlite"}]});
    assert!(decode_completion(completion(bad, "stop")).is_err());
    let schema = proposal_schema();
    let encoded = schema.to_string();
    assert!(!encoded.contains("write"));
    assert!(encoded.contains("distance"));
    assert!(encoded.contains("reference"));
}

#[test]
fn message_window_marks_omitted_history_and_preserves_current_state() {
    let context = DmContext {
        campaign_id: "campaign".into(),
        state_seq: 9,
        state: json!({"rules":"5e-2024","recordedDiceRolls":[{"total":17}]}),
        history: vec![],
        total_turns: 40,
    };
    let payload = messages(&context, "I ask a question.", 0);
    let data: Value = serde_json::from_str(payload[1]["content"].as_str().unwrap()).unwrap();
    assert_eq!(data["earlierTurnsOmitted"], true);
    assert_eq!(
        data["applicationState"]["recordedDiceRolls"][0]["total"],
        17
    );
    assert!(payload[0]["content"]
        .as_str()
        .unwrap()
        .contains("Saving does not end a session"));
}

fn serve_once(response: String) -> (u16, thread::JoinHandle<String>) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let endpoint = listener.local_addr().unwrap().port();
    let thread = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let read = stream.read(&mut buf).unwrap();
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buf[..read]);
            if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                let length = header
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length: "))
                    .and_then(|n| n.parse::<usize>().ok())
                    .unwrap_or(0);
                if bytes.len() >= end + 4 + length {
                    break;
                }
            }
        }
        let _ = stream.write_all(response.as_bytes());
        String::from_utf8_lossy(&bytes).into()
    });
    (endpoint, thread)
}

#[test]
#[ignore = "requires a host that permits local TCP connections; run with --ignored"]
fn transport_authenticates_bounds_response_and_refuses_redirects() {
    let body = r#"{"ok":true}"#;
    let (endpoint, worker) = serve_once(format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ));
    assert_eq!(
        post(endpoint, "launch-secret", "/tokenize", &json!({})).unwrap()["ok"],
        true
    );
    let request = worker.join().unwrap();
    assert!(request
        .to_ascii_lowercase()
        .contains("authorization: bearer launch-secret"));
    let (endpoint,worker)=serve_once("HTTP/1.1 302 Found\r\nLocation: /should-not-follow\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into());
    assert!(post(endpoint, "key", "/tokenize", &json!({}))
        .unwrap_err()
        .contains("302"));
    worker.join().unwrap();
    let body = "x".repeat(RESPONSE_LIMIT as usize + 1);
    let (endpoint, worker) = serve_once(format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ));
    assert!(post(endpoint, "key", "/tokenize", &json!({}))
        .unwrap_err()
        .contains("size limit"));
    worker.join().unwrap();
}

#[test]
fn transport_handles_chunking_and_rejects_truncation_or_ambiguous_framing() {
    assert_eq!(
        decode_http(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n7\r\n{\"x\":1}\r\n0\r\n\r\n"
        )
        .unwrap()["x"],
        1
    );
    for bad in [
        b"HTTP/1.1 200 OK\r\nContent-Length: 99\r\n\r\n{}".as_slice(),
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Length: 2\r\n\r\n0\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nf\r\n{}\r\n",
        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\n{}",
    ] {
        assert!(decode_http(bad).is_err());
    }
    assert!(http_json(
        1,
        "bad\r\nheader",
        "GET",
        "/health",
        None,
        Duration::from_secs(1)
    )
    .is_err());
    assert!(http_json(1, "key", "POST", "/arbitrary", None, Duration::from_secs(1)).is_err());
}

#[test]
fn missing_runtime_never_calls_commit_and_cancellation_has_no_global_process_effects() {
    let dm = LocalDm::new(RuntimeConfig {
        executable: PathBuf::from("missing-llama-server"),
        model: PathBuf::from("missing-model.gguf"),
        log_dir: std::env::temp_dir(),
        gpu_layers: "0".into(),
    });
    let context = DmContext {
        campaign_id: "test".into(),
        state_seq: 1,
        state: json!({}),
        history: vec![],
        total_turns: 0,
    };
    assert!(dm
        .run_turn(&context, "Hello", |_, _| -> Result<(), String> {
            panic!("must not commit")
        })
        .is_err());
    assert!(!dm.status().unwrap().busy);
    assert!(!dm.cancel().unwrap());
    dm.state.lock().unwrap().busy = true;
    assert!(dm.cancel().unwrap());
    assert!(dm.check_cancelled().is_err());
}
