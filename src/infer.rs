use std::num::NonZeroU32;
use std::path::Path;

use anyhow::{bail, Context, Result};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::sampling::LlamaSampler;

const N_CTX: u32 = 4096;
const MAX_PROMPT_TOKENS: usize = 3700;

#[derive(Clone, Copy, clap::ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum TargetLang {
    Zh,
    En,
}

impl TargetLang {
    /// 含 CJK（汉字/日文假名）→ 译成英文，否则译成中文
    pub fn detect(text: &str) -> Self {
        if text.chars().any(is_cjk) {
            TargetLang::En
        } else {
            TargetLang::Zh
        }
    }

    fn name(self) -> &'static str {
        match self {
            TargetLang::Zh => "Simplified Chinese",
            TargetLang::En => "English",
        }
    }
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x3040..=0x30FF)
}

pub struct Translator {
    backend: LlamaBackend,
    model: LlamaModel,
}

impl Translator {
    pub fn load(model_path: &Path, gpu_layers: u32) -> Result<Self> {
        let backend = LlamaBackend::init()?;
        let params = LlamaModelParams::default().with_n_gpu_layers(gpu_layers);
        let model = LlamaModel::load_from_file(&backend, model_path, &params)
            .with_context(|| format!("加载模型失败: {}", model_path.display()))?;
        Ok(Self { backend, model })
    }

    pub fn translate(&self, text: &str, target: TargetLang) -> Result<String> {
        // Qwen3.5-0.8B 默认即非思考模式（且不支持 /no_think 软开关），直接用 ChatML
        let prompt = format!(
            "<|im_start|>system\n\
             You are a professional translator. Translate the user's text into {}. \
             Preserve the original meaning, tone and formatting (including line breaks). \
             Output only the translation, without any explanation.<|im_end|>\n\
             <|im_start|>user\n{}<|im_end|>\n\
             <|im_start|>assistant\n",
            target.name(),
            text
        );

        let vocab = self.model.vocab();
        let tokens = vocab.tokenize(prompt.as_bytes(), true, true);
        if tokens.len() > MAX_PROMPT_TOKENS {
            bail!("文本过长（{} tokens，上限 {MAX_PROMPT_TOKENS}）", tokens.len());
        }

        let threads = std::thread::available_parallelism()
            .map(|n| n.get() as i32)
            .unwrap_or(4);
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(N_CTX))
            .with_n_batch(512)
            .with_n_threads(threads)
            .with_n_threads_batch(threads);
        let mut ctx = self.model.new_context(&self.backend, ctx_params)?;

        let mut batch = LlamaBatch::new(N_CTX as usize, 1);
        batch.add_sequence(&tokens, 0, false)?;
        ctx.decode(&mut batch).context("prompt 处理失败")?;

        // 翻译任务用贪心解码：确定性输出、更忠实原文
        let mut sampler = LlamaSampler::chain_simple([LlamaSampler::greedy()]);

        let max_out = (tokens.len() * 2 + 128).min(2048);
        let mut out_bytes: Vec<u8> = Vec::new();
        let mut pos = tokens.len() as i32;
        for _ in 0..max_out {
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);
            if vocab.is_eog(token) {
                break;
            }
            out_bytes.extend_from_slice(&vocab.token_to_piece(token, false, None));
            batch.clear();
            batch.add(token, pos, &[0], true)?;
            pos += 1;
            ctx.decode(&mut batch).context("解码失败")?;
        }

        let raw = String::from_utf8_lossy(&out_bytes);
        Ok(strip_think(&raw).to_string())
    }
}

/// 兜底剥离 <think>…</think>（/no_think 正常时不会产生）
fn strip_think(s: &str) -> &str {
    let s = s.trim();
    match s.rfind("</think>") {
        Some(end) => s[end + "</think>".len()..].trim(),
        None => s,
    }
}
