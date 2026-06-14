use crate::{
    models::metadata::{ModuleMetadata, ModuleType},
    skills::{MethodInfo, SkillError, SkillModule, SkillOutput},
};

pub struct VoiceSkill {
    metadata: ModuleMetadata,
}

impl VoiceSkill {
    pub fn new() -> Self {
        VoiceSkill {
            metadata: ModuleMetadata {
                name: "Voice".to_string(),
                version: "0.1.0".to_string(),
                _type: ModuleType::Persistent,
                description: "Voice transcription (STT) and speech (TTS) using Groq API and macOS say command".to_string(),
                author: Some("IGRIS".to_string()),
            },
        }
    }

    fn speak_impl(&self, args: &str) -> Result<SkillOutput, SkillError> {
        if args.trim().is_empty() {
            return Err(SkillError::InvalidArgs(
                "No text provided for speech.".to_string(),
            ));
        }

        let status = std::process::Command::new("say")
            .arg(args)
            .status()
            .map_err(|e| {
                SkillError::ExecutionFailed(format!("Failed to run 'say' command: {}", e))
            })?;

        if !status.success() {
            return Err(SkillError::ExecutionFailed(
                "Speech synthesis failed.".to_string(),
            ));
        }

        Ok(SkillOutput::Text(format!("[Voice] Spoke: {}", args)))
    }
}

impl SkillModule for VoiceSkill {
    fn get_metadata(&self) -> &ModuleMetadata {
        &self.metadata
    }

    fn health_check(&self) -> bool {
        std::process::Command::new("which")
            .arg("say")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn execute(&self, method: &str, args: &str) -> Result<SkillOutput, SkillError> {
        match method {
            // "transcribe" => self.transcribe_impl(args),
            "speak" => self.speak_impl(args),
            _ => Err(SkillError::NotFound(format!(
                "Method '{}' not found in Voice skill",
                method
            ))),
        }
    }

    fn available_methods(&self) -> Vec<MethodInfo> {
        vec![
            // MethodInfo {
            //     method: "transcribe".to_string(),
            //     description: "Transcribe audio file using Groq Whisper API. Provide file path or use default /tmp/igris_recording.wav".to_string(),
            //     args_description: "Optional: path to audio file. Example: /tmp/speech.wav".to_string(),
            // },
            MethodInfo {
                method: "speak".to_string(),
                description: "Speak text aloud using macOS 'say' command (TTS).".to_string(),
                args_description: "Text to speak. Example: Hello, how can I help you?".to_string(),
            },
        ]
    }
}
