use crate::{
    models::metadata::{ModuleMetadata, ModuleType},
    skills::{MethodInfo, SkillError, SkillModule, SkillOutput},
};
use serde_json::Value;

pub struct VoiceSkill {
    metadata: ModuleMetadata,
    groq_api_key: Option<String>,
}

impl VoiceSkill {
    pub fn new(groq_api_key: Option<String>) -> Self {
        VoiceSkill {
            metadata: ModuleMetadata {
                name: "Voice".to_string(),
                version: "0.1.0".to_string(),
                _type: ModuleType::Persistent,
                description: "Voice transcription (STT) and speech (TTS) using Groq API and macOS say command".to_string(),
                author: Some("IGRIS".to_string()),
            },
            groq_api_key,
        }
    }

    fn transcribe_impl(&self, args: &str) -> Result<SkillOutput, SkillError> {
        let audio_path = if args.trim().is_empty() {
            "/tmp/igris_recording.wav"
        } else {
            args.trim()
        };
        let api_key = self.groq_api_key.as_ref().ok_or_else(|| {
            SkillError::ExecutionFailed(
                "Groq API key not configured. Add [voice.groq_api_key] to secrets.toml".to_string(),
            )
        })?;

        // Accept file path argument or use default
        if !std::path::Path::new(audio_path).exists() {
            return Err(SkillError::ExecutionFailed(format!(
                "Audio file not found: {}",
                audio_path
            )));
        }

        use base64::Engine;
        use std::io::Read;
        let mut file = std::fs::File::open(audio_path)
            .map_err(|e| SkillError::ExecutionFailed(format!("Cannot open audio file: {}", e)))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| SkillError::ExecutionFailed(format!("Cannot read audio file: {}", e)))?;
        let _b64 = base64::engine::general_purpose::STANDARD.encode(&buffer);

        let boundary = "----boundary_igris_voice";
        let mut body = Vec::new();

        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            "Content-Disposition: form-data; name=\"file\"; filename=\"recording.wav\"\r\n"
                .as_bytes(),
        );
        body.extend_from_slice("Content-Type: audio/wav\r\n\r\n".as_bytes());
        body.extend_from_slice(&buffer);
        body.extend_from_slice(b"\r\n");

        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice("Content-Disposition: form-data; name=\"model\"\r\n\r\n".as_bytes());
        body.extend_from_slice(b"whisper-large-v3\r\n");

        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            "Content-Disposition: form-data; name=\"response_format\"\r\n\r\n".as_bytes(),
        );
        body.extend_from_slice(b"json\r\n");

        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let client = reqwest::blocking::Client::new();
        let response = client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body)
            .send()
            .map_err(|e| SkillError::ExecutionFailed(format!("Groq API request failed: {}", e)))?;

        let text = response.text().map_err(|e| {
            SkillError::ExecutionFailed(format!("Failed to read Groq response: {}", e))
        })?;

        let json: Value = serde_json::from_str(&text).map_err(|e| {
            SkillError::ExecutionFailed(format!("Failed to parse Groq response: {} -> {}", e, text))
        })?;

        if let Some(transcript) = json["text"].as_str() {
            Ok(SkillOutput::Text(format!(
                "[Voice] Transcribed: {}",
                transcript
            )))
        } else if let Some(error) = json["error"]["message"].as_str() {
            Err(SkillError::ExecutionFailed(format!(
                "Groq API error: {}",
                error
            )))
        } else {
            Err(SkillError::ExecutionFailed(format!(
                "Unexpected Groq response: {}",
                text
            )))
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
