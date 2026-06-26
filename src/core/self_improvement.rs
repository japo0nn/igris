use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::error::IgrisError;
use crate::models::assistant::Action;

const COMPILE_TIMEOUT: Duration = Duration::from_secs(300);
const SANDBOX_IMAGE: &str = "rust:latest";
const CARGO_CACHE_VOLUME: &str = "igris_cargo_cache";

/// Буфер для сбора chunk'ов модуля перед компиляцией
#[derive(Debug, Clone)]
pub struct ChunkBuffer {
    pub module_name: String,
    pub total_chunks: u32,
    pub received_chunks: Vec<(u32, String, Vec<String>)>,
    pub is_complete: bool,
}

impl ChunkBuffer {
    pub fn new(module_name: &str, total_chunks: u32) -> Self {
        Self {
            module_name: module_name.to_string(),
            total_chunks,
            received_chunks: Vec::new(),
            is_complete: false,
        }
    }

    /// Добавляет chunk. Возвращает true, если буфер полный.
    pub fn add_chunk(&mut self, index: u32, code: &str, deps: &[String]) -> bool {
        // Избегаем дублирования
        if !self.received_chunks.iter().any(|(i, _, _)| *i == index) {
            self.received_chunks
                .push((index, code.to_string(), deps.to_vec()));
        }
        if self.received_chunks.len() as u32 >= self.total_chunks {
            self.is_complete = true;
            true
        } else {
            false
        }
    }

    /// Собирает полный код из всех chunk'ов (сортируя по индексу)
    pub fn assemble_code(&self) -> String {
        let mut sorted: Vec<_> = self.received_chunks.clone();
        sorted.sort_by_key(|(i, _, _)| *i);
        sorted
            .into_iter()
            .map(|(_, code, _)| code)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Собирает все уникальные зависимости
    pub fn collect_dependencies(&self) -> Vec<String> {
        let mut all_deps: Vec<String> = Vec::new();
        for (_, _, deps) in &self.received_chunks {
            for dep in deps {
                if !all_deps.contains(dep) {
                    all_deps.push(dep.clone());
                }
            }
        }
        all_deps
    }
}

/// Движок самоулучшения IGRIS
fn is_docker_available() -> bool {
    static DOCKER_CHECK: OnceLock<bool> = OnceLock::new();
    *DOCKER_CHECK.get_or_init(|| {
        std::process::Command::new("docker")
            .args(["--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

pub struct SelfImprovementEngine {
    /// Буферы для незавершённых модулей: module_name -> ChunkBuffer
    pub chunk_buffers: Arc<Mutex<HashMap<String, ChunkBuffer>>>,
    /// Директория для сгенерированных модулей
    pub modules_dir: PathBuf,
}

impl SelfImprovementEngine {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let modules_dir = home.join(".igris").join("generated_modules");

        Self {
            chunk_buffers: Arc::new(Mutex::new(HashMap::new())),
            modules_dir,
        }
    }

    /// Обрабатывает GenerateChunk action
    pub async fn handle_chunk(&self, action: &Action) -> Result<Option<String>, IgrisError> {
        match action {
            Action::GenerateChunk {
                module_name,
                chunk_index,
                total_chunks,
                code_chunk,
                dependencies,
            } => {
                let mut buffers = self.chunk_buffers.lock().await;

                // Получаем или создаём буфер для этого модуля
                let is_complete = {
                    let buffer = buffers
                        .entry(module_name.clone())
                        .or_insert_with(|| ChunkBuffer::new(module_name, *total_chunks));

                    // Если total_chunks изменился — обновляем
                    if buffer.total_chunks != *total_chunks {
                        buffer.total_chunks = *total_chunks;
                    }

                    buffer.add_chunk(*chunk_index, code_chunk, dependencies)
                };

                if is_complete {
                    let buffer = buffers.remove(module_name).ok_or_else(|| {
                        IgrisError::InternalError("Buffer disappeared".to_string())
                    })?;

                    // Собираем и компилируем модуль
                    let result = self.build_and_register_module(&buffer).await?;
                    Ok(Some(result))
                } else {
                    Ok(Some(format!(
                        "[CHUNK {}/{}] module={} buffered, waiting for remaining chunks",
                        chunk_index, total_chunks, module_name
                    )))
                }
            }
            _ => Err(IgrisError::InternalError(
                "Not a GenerateChunk action".to_string(),
            )),
        }
    }

    /// Собирает модуль из буфера, записывает файлы, компилирует и регистрирует
    async fn build_and_register_module(&self, buffer: &ChunkBuffer) -> Result<String, IgrisError> {
        let module_name = &buffer.module_name;
        let code = buffer.assemble_code();
        let deps = buffer.collect_dependencies();

        // Создаём директорию модуля
        let module_dir = self.modules_dir.join(module_name);
        let src_dir = module_dir.join("src");
        std::fs::create_dir_all(&src_dir)
            .map_err(|e| IgrisError::IoError(format!("Cannot create module dir: {}", e)))?;

        // Генерируем Cargo.toml
        let mut cargo_toml = format!(
            r#"[package]
                name = "{}"
                version = "0.1.0"
                edition = "2021"

                [dependencies]
                igris_sdk = {{ path = "{}" }}
                "#,
            module_name,
            self.modules_dir
                .parent()
                .unwrap()
                .join("igris_sdk")
                .display()
        );

        // Добавляем зависимости из chunk'а
        for dep in &deps {
            cargo_toml.push_str(&format!("{dep} = \"*\"\n"));
        }

        std::fs::write(module_dir.join("Cargo.toml"), &cargo_toml)
            .map_err(|e| IgrisError::IoError(format!("Cannot write Cargo.toml: {}", e)))?;

        // Генерируем lib.rs
        let lib_content = format!(
            r#"use igris_sdk::{{SkillModule, SkillOutput, SkillError, MethodInfo, ModuleMetadata}};

            {}

            #[no_mangle]
            pub extern "C" fn create_skill() -> Box<dyn SkillModule> {{
                Box::new({}Module)
            }}
            "#,
            code, module_name
        );

        std::fs::write(src_dir.join("lib.rs"), &lib_content)
            .map_err(|e| IgrisError::IoError(format!("Cannot write lib.rs: {}", e)))?;

        // Компилируем модуль
        let output = self.compile_module(&module_dir).await?;

        Ok(format!(
            "[SELF-IMPROVEMENT] Module '{}' built successfully.\nOutput: {}",
            module_name, output
        ))
    }

    /// Запускает cargo build в директории модуля
    async fn compile_module(&self, module_dir: &PathBuf) -> Result<String, IgrisError> {
        if is_docker_available() {
            match self.compile_with_docker(module_dir).await {
                Ok(out) => return Ok(out),
                Err(e) => {
                    eprintln!("[SANDBOX] Docker failed, falling back to local: {}", e);
                }
            }
        }
        self.compile_local(module_dir).await
    }

    async fn compile_with_docker(&self, module_dir: &PathBuf) -> Result<String, IgrisError> {
        let module_path_str = module_dir.to_string_lossy().replace("\\", "/");
        let docker_path = if cfg!(windows)
            && module_path_str.len() >= 3
            && module_path_str.as_bytes()[0].is_ascii_alphabetic()
            && module_path_str.as_bytes()[1] == b':'
        {
            let drive = (module_path_str.as_bytes()[0] as char).to_ascii_lowercase();
            format!("/{}{}", drive, &module_path_str[2..])
        } else {
            module_path_str
        };

        let output = tokio::process::Command::new("docker")
            .args([
                "run",
                "--rm",
                "--cpus",
                "0.5",
                "--memory",
                "256m",
                "--network",
                "none",
                "--read-only",
                "--tmpfs",
                "/tmp:noexec,nosuid,size=64m",
                "--cap-drop",
                "ALL",
                "--security-opt",
                "no-new-privileges",
                "-v",
                &format!("{}:/build", docker_path),
                "-v",
                &format!("{}:/usr/local/cargo/registry", CARGO_CACHE_VOLUME),
                "-w",
                "/build",
                SANDBOX_IMAGE,
                "cargo",
                "build",
                "--release",
            ])
            .output()
            .await
            .map_err(|e| IgrisError::Recoverable(format!("Docker execution error: {}", e)))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(IgrisError::Recoverable(format!(
                "Docker compilation failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }

    async fn compile_local(&self, module_dir: &PathBuf) -> Result<String, IgrisError> {
        let output = tokio::process::Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(module_dir)
            .output()
            .await
            .map_err(|e| IgrisError::IoError(format!("Cargo execution error: {}", e)))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(IgrisError::Recoverable(format!(
                "Compilation failed:\n{}",
                stderr
            )))
        }
    }
}
