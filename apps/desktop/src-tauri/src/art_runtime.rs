use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Mutex,
    },
    time::{Duration, Instant},
};

const WORKER: &str = include_str!("../../../../services/art/worker.py");
const REVISION: &str = "b261bac6fd2cf515557d5d0707481eafa0485ec2";
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtStatus {
    pub available: bool,
    pub detail: String,
    pub model: String,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtRequest {
    pub prompt: String,
    pub seed: u32,
    pub transparent_background: bool,
}
struct Session {
    stdin: ChildStdin,
    lines: Receiver<Result<Value, String>>,
}
pub struct ArtRuntime {
    python: PathBuf,
    model: PathBuf,
    logs: PathBuf,
    child: Mutex<Option<Child>>,
    session: Mutex<Option<Session>>,
    busy: AtomicBool,
    cancelled: AtomicBool,
}
struct Busy<'a>(&'a AtomicBool);
impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
impl ArtRuntime {
    pub fn discover(logs: PathBuf) -> Self {
        let home = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_default();
        let python = std::env::var_os("DWA_ART_PYTHON_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".unsloth/studio/unsloth_studio/Scripts/python.exe"));
        let local = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData/Local"));
        let model = std::env::var_os("DWA_ART_MODEL_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| local.join("DungeonWorldAI/models/sd-turbo"));
        Self {
            python,
            model,
            logs,
            child: Mutex::new(None),
            session: Mutex::new(None),
            busy: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
        }
    }
    pub fn status(&self) -> ArtStatus {
        let marker = fs::read(self.model.join("dwa-model.json"))
            .ok()
            .and_then(|v| serde_json::from_slice::<Value>(&v).ok());
        let required = [
            "model_index.json",
            "scheduler/scheduler_config.json",
            "tokenizer/tokenizer_config.json",
            "tokenizer/vocab.json",
            "tokenizer/merges.txt",
            "text_encoder/model.fp16.safetensors",
            "unet/diffusion_pytorch_model.fp16.safetensors",
            "vae/diffusion_pytorch_model.fp16.safetensors",
        ];
        let ready = self.python.is_file()
            && required.iter().all(|p| self.model.join(p).is_file())
            && marker.as_ref().and_then(|v| v["revision"].as_str()) == Some(REVISION);
        ArtStatus{available:ready,detail:if ready{"Local image model ready. The first generation also loads its weights."}else{"One-time setup needed: open Setup-Local-Art.cmd in the Dungeon-World-AI folder, then restart the app. Uploads work now."}.into(),model:"SD-Turbo · local".into()}
    }
    fn start(&self) -> Result<Session, String> {
        fs::create_dir_all(&self.logs).map_err(|e| e.to_string())?;
        let script = self.logs.join("art-worker.py");
        fs::write(&script, WORKER).map_err(|e| e.to_string())?;
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.logs.join("art-runtime.log"))
            .map_err(|e| e.to_string())?;
        let mut cmd = Command::new(&self.python);
        cmd.args(["-I", "-u"])
            .arg(script)
            .arg(&self.model)
            .env("HF_HUB_OFFLINE", "1")
            .env("TRANSFORMERS_OFFLINE", "1")
            .env("HF_HUB_DISABLE_TELEMETRY", "1")
            .env("DO_NOT_TRACK", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(log);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Could not start the local image worker: {e}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or("Image worker input unavailable.")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Image worker output unavailable.")?;
        *self
            .child
            .lock()
            .map_err(|_| "Image process lock unavailable.")? = Some(child);
        let (tx, lines) = mpsc::sync_channel(8);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = Vec::new();
                match reader.by_ref().take(5_600_001).read_until(b'\n', &mut line) {
                    Ok(0) => break,
                    Ok(_) if line.len() > 5_600_000 => {
                        let _ =
                            tx.send(Err("Image worker response exceeded the size limit.".into()));
                        break;
                    }
                    Ok(_) => {
                        if tx
                            .send(serde_json::from_slice(&line).map_err(|_| {
                                "Image worker returned an unreadable response.".into()
                            }))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                        break;
                    }
                }
            }
        });
        Ok(Session { stdin, lines })
    }
    pub fn generate(&self, request: ArtRequest, progress: impl Fn(&str)) -> Result<Value, String> {
        if self
            .busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("An image is already being generated.".into());
        }
        let _busy = Busy(&self.busy);
        self.cancelled.store(false, Ordering::SeqCst);
        if request.prompt.trim().is_empty() || request.prompt.chars().count() > 1500 {
            return Err("Describe your sprite in up to 1500 characters.".into());
        }
        let status = self.status();
        if !status.available {
            return Err(status.detail);
        }
        let mut session = self
            .session
            .lock()
            .map_err(|_| "Image worker lock unavailable.")?;
        if session.is_none() {
            *session = Some(self.start()?);
        }
        let result = (|| {
            let s = session.as_mut().unwrap();
            writeln!(
                s.stdin,
                "{}",
                serde_json::to_string(&request).map_err(|e| e.to_string())?
            )
            .map_err(|e| e.to_string())?;
            s.stdin.flush().map_err(|e| e.to_string())?;
            let deadline = Instant::now() + Duration::from_secs(180);
            loop {
                if self.cancelled.load(Ordering::SeqCst) {
                    return Err("Image generation cancelled. No asset was saved.".into());
                }
                if Instant::now() > deadline {
                    return Err("Image generation took longer than three minutes. Check logs/art-runtime.log.".into());
                }
                match s.lines.recv_timeout(Duration::from_millis(100)) {
                    Ok(Ok(value)) => match value["type"].as_str() {
                        Some("progress") => {
                            progress(value["message"].as_str().unwrap_or("Drawing…"))
                        }
                        Some("result") => {
                            let url = value["pngDataUrl"]
                                .as_str()
                                .ok_or("Image result is missing its PNG.")?;
                            if !url.starts_with("data:image/png;base64,") || url.len() > 5_590_000 {
                                return Err("Image result is invalid.".into());
                            }
                            return Ok(value);
                        }
                        Some("error") => {
                            return Err(format!(
                                "Local image generation failed: {}",
                                value["message"]
                                    .as_str()
                                    .unwrap_or("Check the runtime log.")
                            ))
                        }
                        _ => return Err("Unexpected image worker response.".into()),
                    },
                    Ok(Err(e)) => return Err(e),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(_) => {
                        return Err(
                            "The local image worker stopped. Check logs/art-runtime.log.".into(),
                        )
                    }
                }
            }
        })();
        if result.is_err() {
            self.stop();
            *session = None;
        }
        result
    }
    fn stop(&self) {
        if let Ok(mut c) = self.child.lock() {
            if let Some(mut child) = c.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    pub fn cancel(&self) -> bool {
        let active = self.busy.load(Ordering::SeqCst);
        if active {
            self.cancelled.store(true, Ordering::SeqCst);
            self.stop();
        }
        active
    }
    pub fn shutdown(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn missing_weights_never_start_a_worker() {
        let mut runtime = ArtRuntime::discover(std::env::temp_dir());
        runtime.model = PathBuf::from("a-model-that-does-not-exist");
        assert!(!runtime.status().available);
        assert!(runtime
            .generate(
                ArtRequest {
                    prompt: "forest bear".into(),
                    seed: 1,
                    transparent_background: true
                },
                |_| {}
            )
            .unwrap_err()
            .contains("Setup-Local-Art"));
        assert!(!runtime.busy.load(Ordering::SeqCst));
        assert!(runtime.child.lock().unwrap().is_none());
    }
    #[test]
    fn requests_are_strict_and_invalid_prompts_fail_before_loading() {
        assert!(serde_json::from_value::<ArtRequest>(
            json!({"prompt":"bear","seed":1,"transparentBackground":true,"url":"x"})
        )
        .is_err());
        let runtime = ArtRuntime::discover(std::env::temp_dir());
        assert!(runtime
            .generate(
                ArtRequest {
                    prompt: " ".into(),
                    seed: 1,
                    transparent_background: true
                },
                |_| {}
            )
            .unwrap_err()
            .contains("Describe"));
    }
}
