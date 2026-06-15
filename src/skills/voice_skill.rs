use crate::{
    models::metadata::{ModuleMetadata, ModuleType},
    skills::{MethodInfo, SkillError, SkillModule, SkillOutput},
};

pub struct VoiceSkill {
    metadata: ModuleMetadata,
}

/// Cross-platform TTS: macOS (say), Linux (espeak/spd-say), Windows (PowerShell SAPI)
fn speak_text(text: &str) -> Result<(), SkillError> {
    if text.trim().is_empty() {
        return Err(SkillError::InvalidArgs("No text provided for speech.".to_string()));
    }

    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("say")
            .arg(text)
            .status()
            .map_err(|e| SkillError::ExecutionFailed(format!("Failed to run 'say': {}", e)))?;
        if !status.success() {
            return Err(SkillError::ExecutionFailed("say command failed".to_string()));
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        // Try espeak first, fallback to spd-say
        let result = std::process::Command::new("espeak")
            .arg(text)
            .status();
        match result {
            Ok(status) if status.success() => return Ok(()),
            _ => {
                // Fallback: spd-say (speech-dispatcher)
                let status = std::process::Command::new("spd-say")
                    .arg(text)
                    .status()
                    .map_err(|e| SkillError::ExecutionFailed(format!("TTS unavailable (try: apt install espeak): {}", e)))?;
                if !status.success() {
                    return Err(SkillError::ExecutionFailed("spd-say failed".to_string()));
                }
            }
        }
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        // Use PowerShell SAPI via Add-Type + SpeakAsync
        let ps_script = format!(
            "Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; $s.SpeakAsync('{}'); $s.Dispose()",
            text.replace("'", "''")
        );
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
            .status()
            .map_err(|e| SkillError::ExecutionFailed(format!("Failed to run PowerShell TTS: {}", e)))?;
        if !status.success() {
            return Err(SkillError::ExecutionFailed("PowerShell TTS failed".to_string()));
        }
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        return Err(SkillError::ExecutionFailed("TTS not supported on this OS".to_string()));
    }
}

impl VoiceSkill {
    pub fn new() -> Self {
        VoiceSkill {
            metadata: ModuleMetadata {
                name: "Voice".to_string(),
                version: "0.1.0".to_string(),
                _type: ModuleType::Persistent,
                description: "Voice speech (TTS) using system commands — macOS say, Linux espeak/spd-say, Windows PowerShell SAPI".to_string(),
                author: Some("IGRIS".to_string()),
            },
        }
    }

    fn speak_impl(&self, args: &str) -> Result<SkillOutput, SkillError> {
        speak_text(args)?;
        Ok(SkillOutput::Text(format!("[Voice] Spoke: {}", args)))
    }
}

impl SkillModule for VoiceSkill {
    fn get_metadata(&self) -> &ModuleMetadata {
        &self.metadata
    }

    fn health_check(&self) -> bool {
        #[cfg(target_os = "macos")]
        { std::process::Command::new("which").arg("say").output().map(|o| o.status.success()).unwrap_or(false) }
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("which").arg("espeak").output().map(|o| o.status.success()).unwrap_or(false)
            || std::process::Command::new("which").arg("spd-say").output().map(|o| o.status.success()).unwrap_or(false)
        }
        #[cfg(target_os = "windows")]
        { true } // PowerShell is always available
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        { false }
    }

    fn execute(&self, method: &str, args: &str) -> Result<SkillOutput, SkillError> {
        match method {
            "speak" => self.speak_impl(args),
            _ => Err(SkillError::NotFound(format!(
                "Method '{}' not found in Voice skill", method
            ))),
        }
    }

    fn available_methods(&self) -> Vec<MethodInfo> {
        vec![
            MethodInfo {
                method: "speak".to_string(),
                description: "Speak text aloud using system TTS (macOS say, Linux espeak/spd-say, Windows PowerShell SAPI).".to_string(),
                args_description: "Text to speak. Example: Hello, how can I help you?".to_string(),
            },
        ]
    }
}
