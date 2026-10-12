use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

pub const DEFAULT_REPO: &str = "unsloth/Qwen3.5-0.8B-GGUF";
pub const DEFAULT_FILE: &str = "Qwen3.5-0.8B-Q4_K_M.gguf";
/// Multimodal projector weights (for image translation)
pub const MMPROJ_FILE: &str = "mmproj-F16.gguf";

#[derive(Clone, Copy, Default, clap::ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum ModelSource {
    #[default]
    ModelScope,
    #[value(alias = "hf")]
    HuggingFace,
}

impl ModelSource {
    fn urls(self, repo: &str, file: &str) -> Vec<String> {
        match self {
            // ModelScope direct link, fast in China
            Self::ModelScope => {
                vec![format!("https://modelscope.cn/models/{repo}/resolve/master/{file}")]
            }
            // China-friendly mirror first, official site as fallback
            Self::HuggingFace => vec![
                format!("https://hf-mirror.com/{repo}/resolve/main/{file}"),
                format!("https://huggingface.co/{repo}/resolve/main/{file}"),
            ],
        }
    }
}

/// Download the model via the system curl (its progress bar goes to stderr, keeping stdout clean).
pub fn download_model(dest: &Path, source: ModelSource) -> Result<()> {
    download_file(dest, source, DEFAULT_FILE)
}

/// Download the mmproj projector weights (for image translation)
pub fn download_mmproj(dest: &Path, source: ModelSource) -> Result<()> {
    download_file(dest, source, MMPROJ_FILE)
}

fn download_file(dest: &Path, source: ModelSource, file: &str) -> Result<()> {
    let dir = dest.parent().expect("model path has no parent directory");
    std::fs::create_dir_all(dir)?;
    let tmp = dest.with_extension("gguf.tmp");

    let mut tried = Vec::new();
    for url in source.urls(DEFAULT_REPO, file) {
        eprintln!("Downloading: {url}");
        let status = Command::new("curl")
            .args(["-fL", "--connect-timeout", "15", "-o"])
            .arg(&tmp)
            .arg(&url)
            .status()
            .context("failed to execute curl (please install curl first)")?;
        if status.success() {
            std::fs::rename(&tmp, dest)?;
            eprintln!("Downloaded: {}", dest.display());
            return Ok(());
        }
        tried.push(url);
    }
    let _ = std::fs::remove_file(&tmp);
    bail!("download failed (tried: {})", tried.join(", "))
}
