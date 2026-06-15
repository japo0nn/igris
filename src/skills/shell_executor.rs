use std::process::Command;

use crate::{
    models::metadata::ModuleMetadata,
    skills::{MethodInfo, SkillError, SkillModule, SkillOutput},
};

#[derive(Debug, Clone)]
pub struct ShellExecutor {
    pub metadata: ModuleMetadata,
}

impl ShellExecutor {
    pub fn new() -> Self {
        ShellExecutor {
            metadata: ModuleMetadata {
                name: "ShellExecutor".to_string(),
                version: "0.1.0".to_string(),
                _type: crate::models::metadata::ModuleType::Persistent,
                description: "Execute shell commands and run programs".to_string(),
                author: Some("IGRIS".to_string()),
            },
        }
    }
}

impl SkillModule for ShellExecutor {
    fn get_metadata(&self) -> &ModuleMetadata {
        &self.metadata
    }

    fn health_check(&self) -> bool {
        true
    }

    fn execute(&self, method: &str, args: &str) -> Result<SkillOutput, SkillError> {
        if method == "execute_command" {
            #[cfg(target_os = "windows")]
            let result = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-OutputFormat",
                    "Text",
                    "-Command",
                    args,
                ])
                .output();

            #[cfg(not(target_os = "windows"))]
            let result = Command::new("sh").args(["-c", args]).output();

            match result {
                Ok(output) => {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        return Err(SkillError::ExecutionFailed(format!(
                            "Command exited with status {}: {}",
                            output.status, stderr
                        )));
                    }

                    if output.stdout.is_empty() {
                        return Ok(SkillOutput::Text(
                            "Command executed successfully (no output)".to_string(),
                        ));
                    }

                    return Ok(SkillOutput::Text(
                        String::from_utf8_lossy(&output.stdout).to_string(),
                    ));

                    #[cfg(not(target_os = "windows"))]
                    {
                        return Ok(SkillOutput::Text(
                            String::from_utf8_lossy(&output.stdout).to_string(),
                        ));
                    }
                }
                Err(e) => {
                    return Err(SkillError::ExecutionFailed(format!(
                        "Failed to execute command: {}",
                        e
                    )));
                }
            }
        } else {
            return Err(SkillError::InvalidArgs("Method does not exist".to_string()));
        }
    }

    fn available_methods(&self) -> Vec<MethodInfo> {
        vec![MethodInfo {
            method: String::from("execute_command"),
            description: String::from("Execute shell commands and programs"),
            args_description: String::from(
                "PowerShell command string on Windows, sh command on Linux/macOS. \
             Example: Get-ChildItem -Path C:\\Users or ls -la /home",
            ),
        }]
    }
}
