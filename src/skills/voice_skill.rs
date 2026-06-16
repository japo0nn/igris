use crate::{
    models::metadata::{ModuleMetadata, ModuleType},
    skills::{MethodInfo, SkillError, SkillModule, SkillOutput},
};

pub struct VoiceSkill {
    metadata: ModuleMetadata,
}

/// Cross-platform TTS - delegates to core::utils::speak_text.
fn speak_text(text: &str) -> Result<(), SkillError> {
    crate::core::utils::speak_text(text).map_err(SkillError::ExecutionFailed)
}

impl VoiceSkill {
    pub fn new() -> Self {
        VoiceSkill {
            metadata: ModuleMetadata {
                name: "Voice".to_string(),
                version: "0.1.0".to_string(),
                _type: ModuleType::Persistent,
                description: "Voice speech (TTS) using system commands - macOS say, Linux espeak/spd-say, Windows PowerShell SAPI".to_string(),
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
        {
            std::process::Command::new("which")
                .arg("say")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("which")
                .arg("espeak")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
                || std::process::Command::new("which")
                    .arg("spd-say")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
        }
        #[cfg(target_os = "windows")]
        {
            true
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            false
        }
    }

    fn execute(&self, method: &str, args: &str) -> Result<SkillOutput, SkillError> {
        match method {
            "speak" => self.speak_impl(args),
            _ => Err(SkillError::NotFound(format!(
                "Method '{}' not found in Voice skill",
                method
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
