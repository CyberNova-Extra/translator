mod download;
mod infer;
mod native_host;

use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::{CommandFactory, Parser};
use download::ModelSource;
use infer::{TargetLang, Translator};

const ENV_MODEL: &str = "QWEN_TRANSLATE_MODEL";

#[derive(Parser)]
#[command(
    name = "translate",
    version,
    about = "本地 Qwen3.5-0.8B 翻译（离线）",
    long_about = "本地 Qwen3.5-0.8B 翻译（离线）。\n\n\
        文本来源：命令行参数优先，否则读 stdin（适合管道组合）。\n\
        模型文件缺失时自动下载（默认 ModelScope）。\n\
        stdout 只输出译文，日志与报错走 stderr。\n\n\
        示例:\n  \
        translate \"I love programming\"\n  \
        echo 我喜欢编程 | translate\n  \
        translate < file.txt"
)]
struct Args {
    /// GGUF 模型路径
    #[arg(long, env = ENV_MODEL)]
    model: Option<PathBuf>,

    /// 模型自动下载源
    #[arg(long, value_enum, default_value = "modelscope")]
    model_source: ModelSource,

    /// 目标语言（默认自动：中文等→英文，其他→中文）
    #[arg(long, value_enum)]
    to: Option<TargetLang>,

    /// 卸载到 GPU 的层数（无 GPU 时自动回退 CPU）
    #[arg(long, default_value_t = 999)]
    gpu_layers: u32,

    /// 以 Firefox 原生消息宿主模式运行（供浏览器扩展调用）
    #[arg(long)]
    native_host: bool,

    /// 要翻译的文本（不给则从 stdin 读取）
    text: Vec<String>,
}

fn default_model_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("qwen-translate")
        .join(download::DEFAULT_FILE)
}

/// 默认路径的模型缺失时自动下载；显式指定的路径缺失则报错
fn ensure_model(model: &Path, explicit: bool, source: ModelSource) -> Result<()> {
    if model.is_file() {
        return Ok(());
    }
    if explicit {
        bail!(
            "找不到模型文件: {}\n（--model / {ENV_MODEL} 显式指定的路径不会自动下载，请检查路径）",
            model.display()
        );
    }
    download::download_model(model, source)
}

fn no_input() -> ! {
    Args::command()
        .error(
            clap::error::ErrorKind::InvalidValue,
            "没有输入文本（用参数或 stdin 传入）",
        )
        .exit()
}

fn run() -> Result<()> {
    let args = Args::parse();

    let (model, model_explicit) = match args.model {
        Some(p) => (p, true),
        None => (default_model_path(), false),
    };

    if args.native_host {
        ensure_model(&model, model_explicit, args.model_source)?;
        let translator = Translator::load(&model, args.gpu_layers)?;
        return native_host::run(&translator);
    }

    let text = if !args.text.is_empty() {
        args.text.join(" ")
    } else if !std::io::stdin().is_terminal() {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        no_input();
    };

    let text = text.trim();
    if text.is_empty() {
        no_input();
    }

    ensure_model(&model, model_explicit, args.model_source)?;
    let translator = Translator::load(&model, args.gpu_layers)?;
    let target = args.to.unwrap_or_else(|| TargetLang::detect(text));
    let out = translator.translate(text, target)?;
    print!("{out}");
    if !out.ends_with('\n') {
        println!();
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("错误: {e:#}");
            ExitCode::FAILURE
        }
    }
}
