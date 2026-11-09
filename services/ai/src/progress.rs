use serde::Serialize;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProgress {
    pub phase: String,
    pub elapsed_seconds: u64,
    pub prompt_tokens: Option<usize>,
    pub processed_tokens: Option<usize>,
    pub generated_tokens: Option<usize>,
}
fn number_after(line: &str, key: &str) -> Option<usize> {
    line.split_once(key)?
        .1
        .trim_start()
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}
pub(crate) fn snapshot(
    path: &Path,
    start: u64,
    phase: &str,
    elapsed: u64,
    total: Option<usize>,
) -> RuntimeProgress {
    let mut progress = RuntimeProgress {
        phase: phase.into(),
        elapsed_seconds: elapsed,
        prompt_tokens: total,
        processed_tokens: None,
        generated_tokens: None,
    };
    if phase != "reading" {
        return progress;
    }
    let read = || -> std::io::Result<String> {
        let mut f = File::open(path)?;
        let size = f.metadata()?.len();
        f.seek(SeekFrom::Start(start.max(size.saturating_sub(16_384))))?;
        let mut bytes = Vec::new();
        f.take(16_384).read_to_end(&mut bytes)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    };
    if let Ok(text) = read() {
        for line in text.lines() {
            if !line.contains(" I slot ") {
                continue;
            }
            if line.contains("prompt processing") {
                progress.processed_tokens = number_after(line, "n_tokens =");
            }
            if line.contains("n_gen =") {
                progress.generated_tokens = number_after(line, "n_gen =");
            }
        }
    }
    if progress.generated_tokens.is_some() {
        progress.phase = "writing".into();
    }
    progress
}
impl RuntimeProgress {
    pub(crate) fn message(&self) -> String {
        let elapsed = if self.elapsed_seconds >= 60 {
            format!(
                "{}m {}s",
                self.elapsed_seconds / 60,
                self.elapsed_seconds % 60
            )
        } else {
            format!("{}s", self.elapsed_seconds)
        };
        let stage = match self.phase.as_str() {
            "loading" => "Loading the local model".to_string(),
            "lookup" => "Checking saved game references".to_string(),
            "writing" => "Writing your reply".to_string(),
            "reading" => match (self.processed_tokens, self.prompt_tokens) {
                (Some(done), Some(total)) if total > 0 => format!(
                    "Reading campaign context — {}%",
                    (done.saturating_mul(100) / total).min(100)
                ),
                _ => "Reading campaign context".into(),
            },
            _ => "Preparing the local reply".into(),
        };
        format!("{stage} · {elapsed}. First replies can take longer on the CPU; Stop is available.")
    }
}
