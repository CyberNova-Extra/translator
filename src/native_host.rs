use std::io::{Read, Write};

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;

use crate::infer::{default_target, Lang, Translator};

const MAX_MSG_LEN: u32 = 8 * 1024 * 1024;

#[derive(Deserialize)]
struct Request {
    text: String,
}

/// Firefox native messaging protocol: each message is a 4-byte little-endian
/// length followed by JSON. Loops forever; the model stays loaded in memory.
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
            Err(e) => return Err(e).context("failed to read message length"),
        }
        let len = u32::from_le_bytes(len_buf);
        if len > MAX_MSG_LEN {
            anyhow::bail!("message too long: {len} bytes");
        }
        let mut buf = vec![0u8; len as usize];
        input.read_exact(&mut buf).context("failed to read message body")?;

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
        Err(e) => return json!({ "error": format!("invalid request: {e}") }),
    };
    let text = req.text.trim();
    if text.is_empty() {
        return json!({ "error": "empty text" });
    }
    let detected = Lang::detect(text);
    match translator.translate(text, None, default_target(detected), false) {
        Ok(t) => json!({ "translation": t }),
        Err(e) => json!({ "error": format!("{e:#}") }),
    }
}
