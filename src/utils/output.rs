use serde::Serialize;
use std::io::Write;

pub enum OutputFormat {
    Text,
    Json,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            _ => OutputFormat::Text,
        }
    }
}

pub fn print_json<T: Serialize>(data: &T) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    println!("{}", json);
    Ok(())
}

pub fn is_json_mode() -> bool {
    std::env::var("FUSION_OUTPUT_FORMAT")
        .map(|v| v.to_lowercase() == "json")
        .unwrap_or(false)
}

pub struct JsonPrinter {
    buffer: Vec<u8>,
}

impl JsonPrinter {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn add_field(&mut self, key: &str, value: serde_json::Value) {
        if !self.buffer.is_empty() {
            self.buffer.write_all(b",\n").ok();
        }
        let line = format!("  \"{}\": {}", key, value);
        self.buffer.write_all(line.as_bytes()).ok();
    }

    pub fn flush(self) -> String {
        let inner = String::from_utf8_lossy(&self.buffer);
        format!("{{\n{}\n}}", inner)
    }
}
