//! test_suffix — 给视频文件名添加后缀
//!
//! 将输入视频文件重命名为 `{stem}{suffix}{ext}`，例如：
//!   alice_20260904_120000.mp4  →  alice_20260904_120000_done.mp4
//!
//! Renames the input video file to `{stem}{suffix}{ext}`, e.g.:
//!   alice_20260904_120000.mp4  →  alice_20260904_120000_done.mp4

use serde::Deserialize;
use std::io::{self, Read};
use std::path::PathBuf;

const DESCRIBE: &str = r#"{
  "description": "Renames the video file by appending a suffix to its stem",
  "inputTypes": ["video_file"],
  "outputTypes": ["video_file"],
  "official": false,
  "params": [
    {
      "key": "suffix",
      "label": "Suffix",
      "type": "string",
      "default": "_done"
    }
  ],
  "i18n": {
    "zh-CN": {
      "description": "在视频文件名末尾（扩展名之前）追加指定后缀",
      "params": {
        "suffix": { "label": "后缀" }
      }
    }
  }
}"#;

const PROGRESS_SCALE: u32 = 10_000;

fn emit_progress(done: u32, total: u32) {
    if total == 0 { return; }
    let scaled = ((done as u64 * PROGRESS_SCALE as u64) / total as u64)
        .min(PROGRESS_SCALE as u64);
    println!("PROGRESS:{}/{}", scaled, PROGRESS_SCALE);
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Input {
    inputs: Vec<String>,
    #[serde(default)]
    params: serde_json::Value,
    #[serde(default)]
    exe_dir: String,
    #[serde(default)]
    max_tmp_mb: u64,
    #[serde(default)]
    recording: Option<serde_json::Value>,
}

impl Input {
    fn param(&self, key: &str, default: &str) -> String {
        self.params
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
            .to_string()
    }
}

fn run() -> serde_json::Value {
    let mut buf = String::new();
    io::stdin().lock().read_to_string(&mut buf).ok();

    let input: Input = match serde_json::from_str(buf.trim()) {
        Ok(v) => v,
        Err(e) => return serde_json::json!({
            "code": "error",
            "message": format!("Failed to parse stdin: {}", e),
            "outputs": []
        }),
    };

    let input_path = match input.inputs.first() {
        Some(p) => PathBuf::from(p),
        None => return serde_json::json!({
            "code": "error",
            "message": "inputs[0] is required",
            "outputs": []
        }),
    };

    if !input_path.exists() {
        return serde_json::json!({
            "code": "error",
            "message": format!("Input not found: {}", input_path.display()),
            "outputs": []
        });
    }

    let suffix = input.param("suffix", "_done");
    if suffix.is_empty() {
        return serde_json::json!({
            "code": "error",
            "message": "suffix param must not be empty",
            "outputs": []
        });
    }

    emit_progress(0, 2);

    // 构造输出路径：在 stem 和扩展名之间插入 suffix
    // Build output path: insert suffix between stem and extension
    let parent = input_path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    let output_name = format!("{}{}{}", stem, suffix, ext);
    let output_path = parent.join(&output_name);

    // 若目标已存在则跳过重命名（幂等）
    // Skip rename if destination already exists (idempotent)
    if output_path.exists() {
        emit_progress(2, 2);
        return serde_json::json!({
            "code": "skipped",
            "message": format!("Output already exists: {}", output_path.display()),
            "outputs": [output_path.to_string_lossy()]
        });
    }

    if let Err(e) = std::fs::rename(&input_path, &output_path) {
        return serde_json::json!({
            "code": "error",
            "message": format!("Rename failed: {}", e),
            "outputs": []
        });
    }

    emit_progress(2, 2);

    eprintln!(
        "[{}] {} → {}",
        env!("CARGO_PKG_NAME"),
        input_path.display(),
        output_path.display()
    );

    serde_json::json!({
        "code": "ok",
        "message": format!("Renamed to {}", output_name),
        "outputs": [output_path.to_string_lossy()]
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("--describe") {
        let pkg_name    = env!("CARGO_PKG_NAME");
        let pkg_version = env!("CARGO_PKG_VERSION");

        let display_name: String = pkg_name
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        let mut desc: serde_json::Value =
            serde_json::from_str(DESCRIBE).expect("DESCRIBE is valid JSON");
        desc["id"]      = serde_json::Value::String(pkg_name.to_string());
        desc["name"]    = serde_json::Value::String(display_name);
        desc["version"] = serde_json::Value::String(pkg_version.to_string());
        print!("{}", desc);
        return;
    }

    let result = run();
    println!("{}", result);
}
