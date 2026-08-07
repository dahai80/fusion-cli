use serde::Serialize;
use std::io::Write;

#[allow(dead_code)]
pub enum OutputFormat {
    Text,
    Json,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
pub struct JsonPrinter {
    buffer: Vec<u8>,
}

#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_is_json_mode_true_when_env_set() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("FUSION_OUTPUT_FORMAT", "json") };
        assert!(is_json_mode());
        unsafe { std::env::remove_var("FUSION_OUTPUT_FORMAT") };
    }

    #[test]
    fn test_is_json_mode_true_case_insensitive() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("FUSION_OUTPUT_FORMAT", "JSON") };
        assert!(is_json_mode());
        unsafe { std::env::remove_var("FUSION_OUTPUT_FORMAT") };
    }

    #[test]
    fn test_is_json_mode_false_when_unset() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("FUSION_OUTPUT_FORMAT") };
        assert!(!is_json_mode());
    }

    #[test]
    fn test_is_json_mode_false_when_non_json() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("FUSION_OUTPUT_FORMAT", "text") };
        assert!(!is_json_mode());
        unsafe { std::env::remove_var("FUSION_OUTPUT_FORMAT") };
    }
}
