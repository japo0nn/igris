use chrono::{DateTime, FixedOffset, Local, NaiveDateTime};

/// Parse a timestamp string stored in the database into DateTime<Local>.
/// Tries several known formats; falls back to Local::now() on failure.
pub fn parse_db_timestamp(value: &str) -> DateTime<Local> {
    if let Ok(dt) = DateTime::<FixedOffset>::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f %:z") {
        return dt.with_timezone(&Local);
    }
    if let Ok(dt) = DateTime::<FixedOffset>::parse_from_str(value, "%Y-%m-%d %H:%M:%S %:z") {
        return dt.with_timezone(&Local);
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        if let chrono::LocalResult::Single(dt) = naive.and_local_timezone(Local) {
            return dt;
        }
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f") {
        if let chrono::LocalResult::Single(dt) = naive.and_local_timezone(Local) {
            return dt;
        }
    }
    Local::now()
}

/// Cross-platform TTS: macOS (say), Linux (espeak/spd-say), Windows (PowerShell SAPI).
pub fn speak_text(text: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("No text provided for speech.".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("say")
            .arg(text)
            .status()
            .map_err(|e| format!("Failed to run say: {}", e))?;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        if std::process::Command::new("espeak")
            .arg(text)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Ok(());
        }
        std::process::Command::new("spd-say")
            .arg(text)
            .status()
            .map_err(|e| format!("TTS unavailable (try: apt install espeak): {}", e))?;
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        let ps = format!(
            "Add-Type -AssemblyName System.Speech; $s=New-Object System.Speech.Synthesis.SpeechSynthesizer; $s.SpeakAsync('{}'); $s.Dispose()",
            text.replace("'", "''")
        );
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
            .status()
            .map_err(|e| format!("Failed to run PowerShell TTS: {}", e))?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err("TTS not supported on this OS".to_string())
    }
}
