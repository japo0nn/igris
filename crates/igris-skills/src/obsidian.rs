use std::{fs, path::PathBuf};

use chrono::Local;

pub struct ObsidianSkill {
    vault_path: PathBuf,
}

impl ObsidianSkill {
    pub fn new(vault_path: &str) -> Self {
        Self {
            vault_path: PathBuf::from(vault_path),
        }
    }

    pub fn write_note(&self, title: &str, content: &str) -> Result<(), String> {
        let igris_dir = self.vault_path.join("IGRIS");

        fs::create_dir_all(&igris_dir)
            .map_err(|e| format!("Не удалось создать папку IGRIS: {}", e))?;

        let date = Local::now().format("%d-%m-%Y").to_string();
        let filename = format!("{}.md", date);
        let filepath = igris_dir.join(&filename);

        let entry = format!("\n## {}\n{}\n", title, content);

        if filepath.exists() {
            let mut existing =
                fs::read_to_string(&filepath).map_err(|e| format!("Ошибка чтения файла: {}", e))?;

            existing.push_str(&entry);
            fs::write(&filepath, existing).map_err(|e| format!("Ошибка записи: {}", e))?;
        } else {
            let new_content = format!("# IGRIS — {}\n{}", date, entry);
            fs::write(&filepath, new_content)
                .map_err(|e| format!("Ошибка создания файла: {}", e))?;
        }

        println!("[Obsidian] Записано в {}", filepath.display());
        Ok(())
    }
}
