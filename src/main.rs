mod download;
mod infer;
mod native_host;

use std::io::{IsTerminal, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Result};
use download::ModelSource;
use infer::{TargetLang, Translator};

const DEFAULT_MODEL_FILE: &str = download::DEFAULT_FILE;
const ENV_MODEL: &str = "QWEN_TRANSLATE_MODEL";

const USAGE: &str = "translate — 本地 Qwen3.5-0.8B 翻译

用法:
  translate [选项] [文本...]     翻译参数中的文本
  echo 文本 | translate          翻译 stdin 的内容
  translate < file.txt           翻译文件内容

模型文件缺失时会自动下载（默认从 ModelScope）。

选项:
  --model <路径>                 GGUF 模型路径（也可用环境变量 QWEN_TRANSLATE_MODEL）
  --model-source <源>            自动下载源: modelscope（默认）| huggingface
  --to <zh|en>                   目标语言（默认自动：中文等→英文，其他→中文）
  --gpu-layers <N>               卸载到 GPU 的层数（默认 999 全部；无 GPU 自动回退 CPU）
  --native-host                  以 Firefox 原生消息宿主模式运行（供浏览器扩展调用）
  -h, --help                     显示本帮助
";

struct Config {
    model: PathBuf,
    model_explicit: bool,
    model_source: ModelSource,
    gpu_layers: u32,
    to: Option<TargetLang>,
    native_host: bool,
    text_parts: Vec<String>,
}

fn default_model_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("qwen-translate")
        .join(DEFAULT_MODEL_FILE)
}

fn parse_args() -> Result<Option<Config>> {
    let mut cfg = Config {
        model: default_model_path(),
        model_explicit: false,
        model_source: ModelSource::default(),
        gpu_layers: 999,
        to: None,
        native_host: false,
        text_parts: Vec::new(),
    };
    if let Some(p) = std::env::var_os(ENV_MODEL) {
        cfg.model = PathBuf::from(p);
        cfg.model_explicit = true;
    }

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let (flag, inline_val) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (arg, None),
        };
        let mut take_value = |name: &str| -> Result<String> {
            match inline_val {
                Some(ref v) => Ok(v.clone()),
                None => args
                    .next()
                    .ok_or_else(|| usage_err(&format!("{name} 需要参数"))),
            }
        };
        match flag.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(None);
            }
            "--native-host" => cfg.native_host = true,
            "--model" => {
                cfg.model = PathBuf::from(take_value("--model")?);
                cfg.model_explicit = true;
            }
            "--model-source" => {
                cfg.model_source = ModelSource::parse(&take_value("--model-source")?)?;
            }
            "--to" => {
                cfg.to = Some(match take_value("--to")?.as_str() {
                    "zh" => TargetLang::Zh,
                    "en" => TargetLang::En,
                    _ => return Err(usage_err("--to 只支持 zh 或 en")),
                });
            }
            "--gpu-layers" => {
                cfg.gpu_layers = take_value("--gpu-layers")?
                    .parse()
                    .map_err(|_| usage_err("--gpu-layers 需要非负整数"))?;
            }
            _ => cfg.text_parts.push(flag),
        }
    }
    Ok(Some(cfg))
}

fn usage_err(msg: &str) -> anyhow::Error {
    anyhow::anyhow!("{msg}\n\n{USAGE}")
}

/// 默认路径的模型缺失时自动下载；显式指定的路径缺失则报错
fn ensure_model(cfg: &Config) -> Result<()> {
    if cfg.model.is_file() {
        return Ok(());
    }
    if cfg.model_explicit {
        bail!(
            "找不到模型文件: {}\n（--model / {ENV_MODEL} 显式指定的路径不会自动下载，请检查路径）",
            cfg.model.display()
        );
    }
    download::download_model(&cfg.model, cfg.model_source)
}

fn run() -> Result<()> {
    let Some(cfg) = parse_args()? else {
        return Ok(());
    };

    if cfg.native_host {
        ensure_model(&cfg)?;
        let translator = Translator::load(&cfg.model, cfg.gpu_layers)?;
        return native_host::run(&translator);
    }

    let text = if !cfg.text_parts.is_empty() {
        cfg.text_parts.join(" ")
    } else if !std::io::stdin().is_terminal() {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        return Err(usage_err("没有输入文本（用参数或 stdin 传入）"));
    };

    let text = text.trim();
    if text.is_empty() {
        return Err(usage_err("输入文本为空"));
    }

    ensure_model(&cfg)?;
    let translator = Translator::load(&cfg.model, cfg.gpu_layers)?;
    let target = cfg.to.unwrap_or_else(|| TargetLang::detect(text));
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
