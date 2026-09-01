use serde::Serialize;

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
