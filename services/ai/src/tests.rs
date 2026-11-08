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
    let schema = proposal_schema(true);
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
#[test]
fn access_denied_is_fatal_at_startup_instead_of_a_false_model_load_timeout() {
    let error = local_connection_error(std::io::Error::from_raw_os_error(10013));
    assert!(error.contains("10013"));
    assert!(error.contains("blocked"));
    assert!(startup_error_is_fatal(&error));
    assert!(startup_error_is_fatal(
        "The local DM rejected the request (HTTP 401)."
    ));
    assert!(!startup_error_is_fatal(&local_connection_error(
        std::io::Error::from(std::io::ErrorKind::ConnectionRefused)
    )));
    assert!(!startup_error_is_fatal(
        "The local DM rejected the request (HTTP 503)."
    ));
}
fn controller_dm() -> LocalDm {
    LocalDm::new(RuntimeConfig {
        executable: PathBuf::from("unused-test-runtime"),
        model: PathBuf::from("unused-test-model.gguf"),
        log_dir: std::env::temp_dir(),
        gpu_layers: "0".into(),
    })
}

fn mixed_lookup() -> Value {
    json!({"schemaVersion":1,"narration":"UNCOMMITTED_DRAFT: a secret door opens before the lookup.",
        "choices":["DRAFT_CHOICE"],"requests":[{"kind":"arithmetic","operation":"add","a":2,"b":3}]})
}

#[test]
fn mixed_lookup_discards_draft_then_saves_only_the_completed_reply_after_real_engine_reads() {
    use dwa_engine::{CampaignStore, DmTurnRequest};
    use std::cell::Cell;
    struct Data(PathBuf);
    impl Drop for Data {
        fn drop(&mut self) {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
            if self.0.starts_with(&root)
                && self
                    .0
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("turn-controller-"))
            {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }
    let data = Data(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("turn-controller-{}", Uuid::new_v4())),
    );
    let store = CampaignStore::new(data.0.join("campaigns")).unwrap();
    let campaign = store.create_campaign("Lookup regression").unwrap().value;
    let request = DmTurnRequest {
        request_id: Uuid::new_v4().to_string(),
        actor_id: None,
        player_text: r#""I follow him into the city to the tavern.""#.into(),
        expected_seq: store
            .load_dm_history(&campaign.id)
            .unwrap()
            .latest_event_seq,
    };
    let context = store.prepare_dm_context(&campaign.id, &request).unwrap();
    let reads = Cell::new(0);
    let completions = Cell::new(0);
    let final_text="You follow the guard to the tavern. At its door, he stops to catch his breath. What do you ask him?";
    let proposal=controller_dm().generate_with_io(&context,&request.player_text,&|tool|{
        assert_eq!(store.load_dm_history(&campaign.id).unwrap().total_turns,0);
        reads.set(reads.get()+1);
        store.read_dm_tool(&campaign.id,context.state_seq,tool).map_err(|e|e.to_string())
    },|route,body|Ok(match route {
        "/apply-template"=>{
            if completions.get()==1 {
                let encoded=body.to_string();assert!(!encoded.contains("UNCOMMITTED_DRAFT"));assert!(!encoded.contains("DRAFT_CHOICE"));
                let current:Value=serde_json::from_str(body["messages"].as_array().unwrap().last().unwrap()["content"].as_str().unwrap()).unwrap();
                assert_eq!(current["readResults"][0]["result"]["result"].as_f64(),Some(5.0));
                assert_eq!(current["playerAction"],request.player_text);
            }
            json!({"prompt":"bounded test prompt"})
        },
        "/tokenize"=>json!({"tokens":[1,2,3]}),
        "/v1/chat/completions"=>{completions.set(completions.get()+1);completion(if completions.get()==1{mixed_lookup()}else{json!({"schemaVersion":1,"narration":final_text,"choices":[],"requests":[]})},"stop")},
        _=>panic!("unexpected route")
    })).unwrap();
    assert_eq!(reads.get(), 1);
    assert_eq!(completions.get(), 2);
    assert_eq!(store.load_dm_history(&campaign.id).unwrap().total_turns, 0);
    store
        .commit_dm_turn(
            &campaign.id,
            &request,
            proposal,
            "test-model",
            PROMPT_VERSION,
        )
        .unwrap();
    drop(store);
    let reopened = CampaignStore::new(data.0.join("campaigns")).unwrap();
    let history = reopened.load_dm_history(&campaign.id).unwrap();
    assert_eq!(history.total_turns, 1);
    assert_eq!(history.turns[0].reply.narration, final_text);
    assert_eq!(history.turns[0].player_text, request.player_text);
    assert!(!serde_json::to_string(&history)
        .unwrap()
        .contains("UNCOMMITTED_DRAFT"));
}

#[test]
fn response_schema_separates_lookups_from_final_prose_and_disables_reads_at_the_limit() {
    let schema = proposal_schema(true);
    let branches = schema["oneOf"].as_array().unwrap();
    assert_eq!(branches.len(), 2);
    assert_eq!(branches[0]["properties"]["narration"]["minLength"], 1);
    assert_eq!(branches[0]["properties"]["requests"]["maxItems"], 0);
    assert_eq!(branches[1]["properties"]["narration"]["const"], "");
    assert_eq!(branches[1]["properties"]["choices"]["maxItems"], 0);
    assert_eq!(branches[1]["properties"]["requests"]["minItems"], 1);
    assert_eq!(branches[1]["properties"]["requests"]["maxItems"], 2);
    let final_only = proposal_schema(false);
    assert!(final_only.get("oneOf").is_none());
    assert_eq!(final_only["properties"]["requests"]["maxItems"], 0);
}

#[test]
fn read_rounds_stay_bounded_even_when_a_runtime_returns_mixed_or_excessive_requests() {
    use std::cell::Cell;
    let context = DmContext {
        campaign_id: "test".into(),
        state_seq: 1,
        state: json!({}),
        history: vec![],
        total_turns: 0,
    };
    let reads = Cell::new(0);
    let completions = Cell::new(0);
    let error = controller_dm()
        .generate_with_io(
            &context,
            "Follow the guard.",
            &|_| {
                reads.set(reads.get() + 1);
                Ok(json!({"result":5}))
            },
            |route, body| {
                Ok(match route {
                    "/apply-template" => json!({"prompt":"test"}),
                    "/tokenize" => json!({"tokens":[1]}),
                    "/v1/chat/completions" => {
                        completions.set(completions.get() + 1);
                        if completions.get() == 4 {
                            assert_eq!(
                                body["response_format"]["schema"]["properties"]["requests"]
                                    ["maxItems"],
                                0
                            );
                        }
                        let mut reply = mixed_lookup();
                        let tool = reply["requests"][0].clone();
                        reply["requests"].as_array_mut().unwrap().push(tool);
                        completion(reply, "stop")
                    }
                    _ => panic!("unexpected route"),
                })
            },
        )
        .unwrap_err();
    assert!(error.contains("lookup allowance"));
    assert_eq!(reads.get(), 6);
    assert_eq!(completions.get(), 4);
    let error = controller_dm()
        .generate_with_io(
            &context,
            "Follow the guard.",
            &|_| panic!("must not execute an oversized batch"),
            |route, _| {
                Ok(match route {
                    "/apply-template" => json!({"prompt":"test"}),
                    "/tokenize" => json!({"tokens":[1]}),
                    "/v1/chat/completions" => {
                        let mut reply = mixed_lookup();
                        let tool = reply["requests"][0].clone();
                        reply["requests"] = json!([tool, tool, tool]);
                        completion(reply, "stop")
                    }
                    _ => panic!("unexpected route"),
                })
            },
        )
        .unwrap_err();
    assert!(error.contains("too many lookups"));
}

#[test]
fn saved_conversation_roles_keep_the_latest_action_separate_from_the_opening_scene() {
    let context = DmContext {
        campaign_id: "campaign".into(),
        state_seq: 9,
        state: json!({"authoritative":"current snapshot"}),
        total_turns: 1,
        history: vec![dwa_engine::DmTurn {
            id: "turn".into(),
            campaign_id: "campaign".into(),
            request_id: "request".into(),
            actor_id: None,
            player_text: "Begin at the gate.".into(),
            reply: DmProposal {
                schema_version: 1,
                narration: "OPENING_SCENE_CANARY".into(),
                choices: vec!["Follow the guard.".into()],
            },
            model: "test".into(),
            prompt_version: PROMPT_VERSION.into(),
            based_on_seq: 8,
            event_seq: 9,
            created_at: "2026-11-08T10:00:00Z".into(),
        }],
    };
    let payload = messages(&context, "Follow him into the tavern.", 0);
    let items = payload.as_array().unwrap();
    assert_eq!(items.len(), 4);
    assert_eq!(items[1]["role"], "user");
    assert_eq!(items[2]["role"], "assistant");
    assert_eq!(items[3]["role"], "user");
    let current: Value = serde_json::from_str(items[3]["content"].as_str().unwrap()).unwrap();
    assert_eq!(current["playerAction"], "Follow him into the tavern.");
    assert_eq!(
        current["applicationState"]["authoritative"],
        "current snapshot"
    );
    assert_eq!(current["earlierTurnsOmitted"], false);
    assert!(!items[3]["content"]
        .as_str()
        .unwrap()
        .contains("OPENING_SCENE_CANARY"));
    let trimmed = messages(&context, "Follow him into the tavern.", 1);
    assert_eq!(trimmed.as_array().unwrap().len(), 2);
    let current: Value = serde_json::from_str(trimmed[1]["content"].as_str().unwrap()).unwrap();
    assert_eq!(current["earlierTurnsOmitted"], true);
    assert_eq!(current["stateRevision"], 9);
}
