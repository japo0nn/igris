use serde_json::Value;
use crate::{
    models::metadata::{ModuleMetadata, ModuleType},
    skills::{MethodInfo, SkillModule, SkillOutput, SkillError},
};

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
                description: "Voice recording, transcription (STT) and speech (TTS) using system tools and Groq API".to_string(),
                author: Some("IGRIS".to_string()),
            },
            groq_api_key,
        }
    }

    fn record_impl(&self, _args: &str) -> Result<SkillOutput, SkillError> {
        let output_path = "/tmp/igris_recording.wav";

        // Check which recording tool is available
        let rec_cmd = if std::process::Command::new("ffmpeg")
            .arg("-version")
            .output()
            .is_ok()
        {
            format!(
                "ffmpeg -f avfoundation -i ':default' -t 5 -y {} 2>/dev/null",
                output_path
            )
        } else if std::process::Command::new("sox").arg("--version").output().is_ok() {
            format!("sox -d -t wav {} trim 0 5", output_path)
        } else {
            return Err(SkillError::ExecutionFailed(
                "No recording tool found. Install ffmpeg (brew install ffmpeg) or sox (brew install sox)".to_string()
            ));
        };

        eprintln!("[Voice] Recording for 5 seconds...");
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&rec_cmd)
            .status()
            .map_err(|e| SkillError::ExecutionFailed(format!("Failed to start recording: {}", e)))?;

        if !status.success() {
            return Err(SkillError::ExecutionFailed(
                "Recording failed. Check microphone permissions.".to_string()
            ));
        }

        if !std::path::Path::new(output_path).exists() {
            return Err(SkillError::ExecutionFailed(
                "Recording file was not created.".to_string()
            ));
        }

        let size = std::fs::metadata(output_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(SkillOutput::Text(format!(
            "[Voice] Recording saved to {} ({} bytes, ~5 seconds)",
            output_path, size
        )))
    }

    fn transcribe_impl(&self, _args: &str) -> Result<SkillOutput, SkillError> {
        let audio_path = "/tmp/igris_recording.wav";
        let api_key = self.groq_api_key.as_ref().ok_or_else(|| {
            SkillError::ExecutionFailed(
                "Groq API key not configured. Add [voice.groq_api_key] to secrets.toml".to_string()
            )
        })?;

        if !std::path::Path::new(audio_path).exists() {
            return Err(SkillError::ExecutionFailed(
                "No recording found at /tmp/igris_recording.wav. Call record() first.".to_string()
            ));
        }

        // Read audio file as base64
        use base64::Engine;
        use std::io::Read;
        let mut file = std::fs::File::open(audio_path)
            .map_err(|e| SkillError::ExecutionFailed(format!("Cannot open audio file: {}", e)))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| SkillError::ExecutionFailed(format!("Cannot read audio file: {}", e)))?;
        let _b64 = base64::engine::general_purpose::STANDARD.encode(&buffer);

        // Create multipart form
        let boundary = "----boundary_igris_voice";
        let mut body = Vec::new();

        // Add file part
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            "Content-Disposition: form-data; name=\"file\"; filename=\"recording.wav\"\r\n"
                .as_bytes(),
        );
        body.extend_from_slice("Content-Type: audio/wav\r\n\r\n".as_bytes());
        body.extend_from_slice(&buffer);
        body.extend_from_slice(b"\r\n");

        // Add model part
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            "Content-Disposition: form-data; name=\"model\"\r\n\r\n"
                .as_bytes(),
        );
        body.extend_from_slice(b"whisper-large-v3\r\n");

        // Add response_format part
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            "Content-Disposition: form-data; name=\"response_format\"\r\n\r\n"
                .as_bytes(),
        );
        body.extend_from_slice(b"json\r\n");

        // Close
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        // Send request
        let client = reqwest::blocking::Client::new();
        let response = client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
            .body(body)
            .send()
            .map_err(|e| SkillError::ExecutionFailed(format!("Groq API request failed: {}", e)))?;

        let text = response.text()
            .map_err(|e| SkillError::ExecutionFailed(format!("Failed to read Groq response: {}", e)))?;

        let json: Value = serde_json::from_str(&text)
            .map_err(|e| SkillError::ExecutionFailed(format!("Failed to parse Groq response: {} -> {}", e, text)))?;

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
                "No text provided for speech.".to_string()
            ));
        }

        let status = std::process::Command::new("say")
            .arg(args)
            .status()
            .map_err(|e| SkillError::ExecutionFailed(format!("Failed to run 'say' command: {}", e)))?;

        if !status.success() {
            return Err(SkillError::ExecutionFailed(
                "Speech synthesis failed.".to_string()
            ));
        }

        Ok(SkillOutput::Text(format!(
            "[Voice] Spoke: {}",
            args
        )))
    }
}

impl SkillModule for VoiceSkill {
    fn get_metadata(&self) -> &ModuleMetadata {
        &self.metadata
    }

    fn health_check(&self) -> bool {
        // Check if 'say' exists (macOS) or other TTS
        std::process::Command::new("which")
            .arg("say")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn execute(&self, method: &str, args: &str) -> Result<SkillOutput, SkillError> {
        match method {
            "record" => self.record_impl(args),
            "transcribe" => self.transcribe_impl(args),
            "speak" => self.speak_impl(args),
            _ => Err(SkillError::NotFound(format!("Method '{}' not found in Voice skill", method))),
        }
    }

    fn available_methods(&self) -> Vec<MethodInfo> {
        vec![
            MethodInfo {
                method: "record".to_string(),
                description: "Record audio from microphone for 5 seconds. Saves to /tmp/igris_recording.wav".to_string(),
                args_description: "No arguments required. Pass an empty string.".to_string(),
            },
            MethodInfo {
                method: "transcribe".to_string(),
                description: "Transcribe the last recorded audio using Groq Whisper API. Requires groq_api_key in secrets.toml".to_string(),
                args_description: "No arguments required. Pass an empty string.".to_string(),
            },
            MethodInfo {
                method: "speak".to_string(),
                description: "Speak text aloud using macOS 'say' command (TTS).".to_string(),
                args_description: "Text to speak. Example: Hello, how can I help you?".to_string(),
            },
        ]
    }
}
