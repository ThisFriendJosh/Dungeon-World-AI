use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScryingStatus {
    pub available: bool,
    pub running: bool,
    pub ready: bool,
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub error: Option<String>,
    pub cef_version: Option<String>,
    pub chromium_version: Option<String>,
}
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PaneBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
impl PaneBounds {
    fn valid(&self) -> bool {
        self.x.unsigned_abs() <= 20000
            && self.y.unsigned_abs() <= 20000
            && (1..=16000).contains(&self.width)
            && (1..=16000).contains(&self.height)
    }
}
struct BrowserProcess {
    child: Child,
    input: ChildStdin,
    #[cfg(windows)]
    _job: ProcessJob,
}
#[cfg(windows)]
struct ProcessJob(isize);
#[cfg(windows)]
impl Drop for ProcessJob {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0 as _);
        }
    }
}
#[cfg(windows)]
impl ProcessJob {
    fn attach(child: &Child) -> Result<Self, String> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let raw = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if raw.is_null() {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let job = Self(raw as isize);
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                raw,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as _,
                std::mem::size_of_val(&limits) as u32,
            ) == 0
                || AssignProcessToJobObject(raw, child.as_raw_handle() as _) == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
            Ok(job)
        }
    }
}
pub struct ScryingRuntime {
    roots: Vec<PathBuf>,
    data: PathBuf,
    process: Mutex<Option<BrowserProcess>>,
    state: Arc<Mutex<ScryingStatus>>,
    epoch: Arc<AtomicU64>,
}
impl ScryingRuntime {
    pub fn new(resources: PathBuf, data: PathBuf) -> Self {
        let mut roots = vec![resources.join("scrying-cef")];
        if let Some(build) = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent()?.parent().map(|p| p.to_path_buf()))
        {
            roots.push(build.join("scrying-cef"));
        }
        Self {
            roots,
            data,
            process: Mutex::new(None),
            state: Arc::new(Mutex::new(ScryingStatus::default())),
            epoch: Arc::new(AtomicU64::new(0)),
        }
    }
    fn executable(&self) -> Option<PathBuf> {
        if let Some(path) = std::env::var_os("DWA_CEF_BROWSER_PATH")
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
        {
            return path.is_file().then_some(path);
        }
        for root in &self.roots {
            let value = fs::read(root.join("current.json"))
                .ok()
                .and_then(|s| serde_json::from_slice::<Value>(&s).ok());
            if let Some(folder) = value
                .as_ref()
                .and_then(|v| v["directory"].as_str())
                .filter(|s| {
                    !s.is_empty()
                        && !s.contains("..")
                        && s.bytes()
                            .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-')
                })
            {
                let executable = root.join(folder).join("ScryingPane.exe");
                if executable.is_file() && executable.with_file_name("libcef.dll").is_file() {
                    return Some(executable);
                }
            }
        }
        None
    }
    pub fn status(&self) -> Result<ScryingStatus, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Browser state lock unavailable.")?
            .clone();
        state.available = cfg!(windows) && self.executable().is_some();
        if !state.available {
            state.error =
                Some("Open Setup-Scrying-Pane.cmd once to install the CEF browser runtime.".into());
        }
        Ok(state)
    }
    fn send(process: &mut BrowserProcess, value: &Value) -> Result<(), String> {
        writeln!(process.input, "{value}")
            .and_then(|_| process.input.flush())
            .map_err(|e| format!("The browser connection stopped: {e}"))
    }
    pub fn open(&self, parent: isize, url: &str, bounds: PaneBounds) -> Result<(), String> {
        #[cfg(not(windows))]
        {
            let _ = (parent, url, bounds);
            return Err("CEF embedding currently requires Windows.".into());
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            if !bounds.valid() || parent == 0 {
                return Err("The browser pane has invalid bounds.".into());
            }
            let url = dwa_engine::validated_browser_url(url).map_err(|e| e.to_string())?;
            let mut process = self
                .process
                .lock()
                .map_err(|_| "Browser process lock unavailable.")?;
            if process
                .as_mut()
                .is_some_and(|p| p.child.try_wait().ok().flatten().is_some())
            {
                *process = None;
            }
            if process.is_none() {
                let executable = self
                    .executable()
                    .ok_or("Open Setup-Scrying-Pane.cmd once to install the CEF runtime.")?;
                let profile = self.data.join("scrying-profile");
                let logs = self.data.join("logs");
                fs::create_dir_all(&profile)
                    .and_then(|_| fs::create_dir_all(&logs))
                    .map_err(|e| e.to_string())?;
                let log = fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(logs.join("scrying-host.log"))
                    .map_err(|e| e.to_string())?;
                let mut child = Command::new(&executable)
                    .current_dir(
                        executable
                            .parent()
                            .ok_or("Browser directory unavailable.")?,
                    )
                    .arg("--parent")
                    .arg(parent.to_string())
                    .arg("--parent-pid")
                    .arg(std::process::id().to_string())
                    .arg("--profile")
                    .arg(profile)
                    .arg("--log")
                    .arg(logs.join("scrying-cef.log"))
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(log)
                    .creation_flags(0x08000000)
                    .spawn()
                    .map_err(|e| format!("Could not start CEF: {e}"))?;
                let job = match ProcessJob::attach(&child) {
                    Ok(job) => job,
                    Err(e) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(format!("Could not own the browser process: {e}"));
                    }
                };
                let input = child.stdin.take().ok_or("Browser input unavailable.")?;
                let output = child.stdout.take().ok_or("Browser output unavailable.")?;
                let epoch = self.epoch.fetch_add(1, Ordering::SeqCst) + 1;
                let generation = self.epoch.clone();
                let shared = self.state.clone();
                *self
                    .state
                    .lock()
                    .map_err(|_| "Browser state lock unavailable.")? = ScryingStatus {
                    available: true,
                    running: true,
                    url: url.clone(),
                    title: "Scrying Pane".into(),
                    loading: true,
                    ..Default::default()
                };
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(output);
                    loop {
                        let mut line = vec![];
                        match reader.by_ref().take(16385).read_until(b'\n', &mut line) {
                            Ok(0) => break,
                            Ok(_) if line.len() <= 16384 => {
                                if generation.load(Ordering::SeqCst) != epoch {
                                    break;
                                }
                                if let Ok(value) = serde_json::from_slice::<Value>(&line) {
                                    if let Ok(mut state) = shared.lock() {
                                        if value["type"] == "state" {
                                            state.ready = value["ready"].as_bool().unwrap_or(false);
                                            state.loading =
                                                value["loading"].as_bool().unwrap_or(false);
                                            state.can_go_back =
                                                value["canGoBack"].as_bool().unwrap_or(false);
                                            state.can_go_forward =
                                                value["canGoForward"].as_bool().unwrap_or(false);
                                            state.url = value["url"]
                                                .as_str()
                                                .unwrap_or("")
                                                .chars()
                                                .take(2048)
                                                .collect();
                                            state.title = value["title"]
                                                .as_str()
                                                .unwrap_or("")
                                                .chars()
                                                .take(120)
                                                .collect();
                                            state.cef_version = value["cefVersion"]
                                                .as_str()
                                                .map(|s| s.chars().take(80).collect());
                                            state.chromium_version = value["chromiumVersion"]
                                                .as_str()
                                                .map(|s| s.chars().take(80).collect());
                                        }
                                        state.error = value["error"]
                                            .as_str()
                                            .filter(|s| !s.is_empty())
                                            .map(|s| s.chars().take(1500).collect());
                                    }
                                }
                            }
                            _ => break,
                        }
                    }
                    if generation.load(Ordering::SeqCst) == epoch {
                        if let Ok(mut state) = shared.lock() {
                            state.running = false;
                            state.ready = false;
                            state.loading = false;
                            if state.error.is_none() {
                                state.error =
                                    Some("The browser closed. Use Go to open it again.".into());
                            }
                        }
                    }
                });
                *process = Some(BrowserProcess {
                    child,
                    input,
                    _job: job,
                });
            }
            let process = process.as_mut().unwrap();
            Self::send(
                process,
                &json!({"kind":"layout","x":bounds.x,"y":bounds.y,"width":bounds.width,"height":bounds.height}),
            )?;
            Self::send(process, &json!({"kind":"navigate","url":url}))
        }
    }
    pub fn layout(&self, bounds: PaneBounds) -> Result<(), String> {
        if !bounds.valid() {
            return Err("Invalid browser bounds.".into());
        }
        let mut process = self
            .process
            .lock()
            .map_err(|_| "Browser process lock unavailable.")?;
        if let Some(process) = process.as_mut() {
            Self::send(
                process,
                &json!({"kind":"layout","x":bounds.x,"y":bounds.y,"width":bounds.width,"height":bounds.height}),
            )?;
        }
        Ok(())
    }
    pub fn command(&self, kind: &str) -> Result<(), String> {
        if !matches!(kind, "back" | "forward" | "reload" | "stop" | "hide") {
            return Err("Unknown browser command.".into());
        }
        let mut process = self
            .process
            .lock()
            .map_err(|_| "Browser process lock unavailable.")?;
        if let Some(process) = process.as_mut() {
            Self::send(process, &json!({"kind":kind}))?;
        }
        Ok(())
    }
    pub fn shutdown(&self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut process) = self.process.lock() {
            if let Some(mut owned) = process.take() {
                let _ = Self::send(&mut owned, &json!({"kind":"close"}));
                let deadline = Instant::now() + Duration::from_secs(2);
                while Instant::now() < deadline {
                    if owned.child.try_wait().ok().flatten().is_some() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let _ = owned.child.kill();
                let _ = owned.child.wait();
            }
        }
        if let Ok(mut state) = self.state.lock() {
            *state = ScryingStatus::default();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pane_bounds_are_bounded_before_native_window_operations() {
        assert!(PaneBounds {
            x: 30,
            y: 200,
            width: 900,
            height: 500
        }
        .valid());
        assert!(!PaneBounds {
            x: i32::MIN,
            y: 0,
            width: 900,
            height: 500
        }
        .valid());
        assert!(!PaneBounds {
            x: 0,
            y: 0,
            width: 0,
            height: 500
        }
        .valid());
    }
    #[test]
    fn no_browser_process_is_started_by_status_or_unknown_commands() {
        let runtime = ScryingRuntime::new(PathBuf::from("missing"), std::env::temp_dir());
        let _ = runtime.status().unwrap();
        assert!(runtime.command("evaluateJavaScript").is_err());
        assert!(runtime.process.lock().unwrap().is_none());
    }
}
