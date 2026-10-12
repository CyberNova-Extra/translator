use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

pub const DEFAULT_REPO: &str = "unsloth/Qwen3.5-0.8B-GGUF";
pub const DEFAULT_FILE: &str = "Qwen3.5-0.8B-Q4_K_M.gguf";

#[derive(Clone, Copy, Default)]
pub enum ModelSource {
    #[default]
    ModelScope,
    HuggingFace,
}

impl ModelSource {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "modelscope" => Ok(Self::ModelScope),
            "huggingface" | "hf" => Ok(Self::HuggingFace),
            _ => bail!("--model-source 只支持 modelscope 或 huggingface"),
        }
    }

    fn urls(self, repo: &str, file: &str) -> Vec<String> {
        match self {
            // ModelScope 直链，国内速度快
            Self::ModelScope => {
                vec![format!("https://modelscope.cn/models/{repo}/resolve/master/{file}")]
            }
            // 优先国内镜像，兜底官方站
            Self::HuggingFace => vec![
                format!("https://hf-mirror.com/{repo}/resolve/main/{file}"),
                format!("https://huggingface.co/{repo}/resolve/main/{file}"),
            ],
        }
    }
}

/// 用系统 curl 下载模型（curl 进度条天然走 stderr，不污染 stdout）。
pub fn download_model(dest: &Path, source: ModelSource) -> Result<()> {
    let dir = dest.parent().expect("模型路径无父目录");
    std::fs::create_dir_all(dir)?;
    let tmp = dest.with_extension("gguf.tmp");

    let mut tried = Vec::new();
    for url in source.urls(DEFAULT_REPO, DEFAULT_FILE) {
        eprintln!("下载模型: {url}");
        let status = Command::new("curl")
            .args(["-fL", "--connect-timeout", "15", "-o"])
            .arg(&tmp)
            .arg(&url)
            .status()
            .context("无法执行 curl（请先安装 curl）")?;
        if status.success() {
            std::fs::rename(&tmp, dest)?;
            eprintln!("模型已下载到: {}", dest.display());
            return Ok(());
        }
        tried.push(url);
    }
    let _ = std::fs::remove_file(&tmp);
    bail!("模型下载失败（已尝试: {}）", tried.join(", "))
}
