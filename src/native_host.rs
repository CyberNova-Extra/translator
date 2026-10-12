use std::io::{Read, Write};

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;

use crate::infer::{TargetLang, Translator};

const MAX_MSG_LEN: u32 = 8 * 1024 * 1024;

#[derive(Deserialize)]
struct Request {
    text: String,
}

/// Firefox 原生消息协议：每条消息 = 4 字节小端长度 + JSON。
/// 循环处理，模型常驻内存。
pub fn run(translator: &Translator) -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    loop {
        let mut len_buf = [0u8; 4];
        match input.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e).context("读取消息长度失败"),
        }
        let len = u32::from_le_bytes(len_buf);
        if len > MAX_MSG_LEN {
            anyhow::bail!("消息过长: {len} 字节");
        }
        let mut buf = vec![0u8; len as usize];
        input.read_exact(&mut buf).context("读取消息体失败")?;

        let response = handle(translator, &buf);
        let payload = serde_json::to_vec(&response)?;
        output.write_all(&(payload.len() as u32).to_le_bytes())?;
        output.write_all(&payload)?;
        output.flush()?;
    }
}

fn handle(translator: &Translator, buf: &[u8]) -> serde_json::Value {
    let req = match serde_json::from_slice::<Request>(buf) {
        Ok(r) => r,
        Err(e) => return json!({ "error": format!("请求格式错误: {e}") }),
    };
    let text = req.text.trim();
    if text.is_empty() {
        return json!({ "error": "文本为空" });
    }
    match translator.translate(text, TargetLang::detect(text)) {
        Ok(t) => json!({ "translation": t }),
        Err(e) => json!({ "error": format!("{e:#}") }),
    }
}
