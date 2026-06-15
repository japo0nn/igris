use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::models::metadata::{ModuleMetadata, ModuleType};
use crate::skills::{MethodInfo, SkillError, SkillModule, SkillOutput};

pub struct UserProfileSkill {
    pub metadata: ModuleMetadata,
    profile_path: PathBuf,
    pub profile: Profile,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Profile {
    pub preferences: HashMap<String, String>,
    pub music_genre: Option<String>,
    pub favorite_topics: Vec<String>,
    pub language: String,
    pub investment_preferences: InvestmentPrefs,
    pub last_updated: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct InvestmentPrefs {
    pub risk_tolerance: String,
    pub invested_amount: f64,
    pub interested_sectors: Vec<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            preferences: HashMap::new(),
            music_genre: None,
            favorite_topics: vec![
                "news".to_string(),
                "technology".to_string(),
                "investment".to_string(),
            ],
            language: "russian".to_string(),
            investment_preferences: InvestmentPrefs {
                risk_tolerance: "medium".to_string(),
                invested_amount: 50000.0,
                interested_sectors: vec![
                    "cryptocurrency".to_string(),
                    "ai".to_string(),
                    "space".to_string(),
                ],
            },
            last_updated: String::new(),
        }
    }
}

impl UserProfileSkill {
    pub fn new() -> Self {
        let profile_path = dirs::home_dir()
            .map(|p| p.join(".igris").join("user_profile.json"))
            .unwrap_or_else(|| std::env::temp_dir().join("igris_user_profile.json"));

        let profile = if profile_path.exists() {
            fs::read_to_string(&profile_path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            Profile::default()
        };

        Self {
            metadata: ModuleMetadata {
                name: "UserProfileSkill".to_string(),
                description: "Manages persistent user profile with preferences and habits"
                    .to_string(),
                version: "0.1.0".to_string(),
                _type: ModuleType::Persistent,
                author: None,
            },
            profile_path,
            profile,
        }
    }

    pub fn save(&self) -> Result<(), SkillError> {
        if let Some(parent) = self.profile_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                SkillError::ExecutionFailed(format!("Failed to create profile dir: {}", e))
            })?;
        }
        let json = serde_json::to_string_pretty(&self.profile)
            .map_err(|e| SkillError::ExecutionFailed(format!("Serialization error: {}", e)))?;
        fs::write(&self.profile_path, json)
            .map_err(|e| SkillError::ExecutionFailed(format!("Failed to write profile: {}", e)))?;
        Ok(())
    }

    pub fn update_preference(&mut self, key: &str, value: &str) -> Result<(), SkillError> {
        self.profile
            .preferences
            .insert(key.to_string(), value.to_string());
        self.profile.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();
        self.save()
    }
}

impl SkillModule for UserProfileSkill {
    fn get_metadata(&self) -> &ModuleMetadata {
        &self.metadata
    }

    fn health_check(&self) -> bool {
        true
    }

    fn available_methods(&self) -> Vec<MethodInfo> {
        vec![
            MethodInfo {
                method: "get-profile".to_string(),
                description: "Return the full user profile as JSON string".to_string(),
                args_description: "No arguments required".to_string(),
            },
            MethodInfo {
                method: "update-preference".to_string(),
                description: "Update a single user preference by key-value pair".to_string(),
                args_description:
                    "Key and value separated by a pipe '|'. Example: music_genre|jazz".to_string(),
            },
            MethodInfo {
                method: "add-topic".to_string(),
                description: "Add a new topic of interest".to_string(),
                args_description: "Topic name. Example: rust".to_string(),
            },
        ]
    }

    fn execute(&self, method: &str, args: &str) -> Result<SkillOutput, SkillError> {
        match method {
            "get-profile" => {
                let json = serde_json::to_value(&self.profile).map_err(|e| {
                    SkillError::ExecutionFailed(format!("Serialization error: {}", e))
                })?;
                Ok(SkillOutput::Json(json))
            }
            "update-preference" => Ok(SkillOutput::Text(format!(
                "Preference update requested: {}",
                args
            ))),
            "add-topic" => Ok(SkillOutput::Text(format!("Topic add requested: {}", args))),
            _ => Err(SkillError::NotFound(method.to_string())),
        }
    }
}
