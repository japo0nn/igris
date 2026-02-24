use chrono::Utc;
use igris_skills::obsidian::ObsidianSkill;
use std::io::{self, Write};

use igris_core::{
    config::load_config,
    llm::{LlmAction, send_message},
};
use igris_memory::{db::MemoryDb, models::ChatMessage};

fn main() {
    let config = load_config("config.toml").unwrap_or_else(|e| {
        eprintln!("Ошибка загрузки конфига: {}", e);
        std::process::exit(1);
    });

    println!("IGRIS запущен введите команду:");

    let memory_db = MemoryDb::open("igris.db").expect("Не удалось открыть базу igris.db");
    let session_id = MemoryDb::create_session(&memory_db).expect("Error creating session");
    let obsidian = ObsidianSkill::new(&config.integrations.obsidian_vault);
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input == "exit" {
            break;
        }

        process_message(
            &memory_db,
            &session_id,
            ChatMessage {
                role: "user".to_string(),
                content: input.to_string(),
                timestamp: Utc::now().timestamp().to_string(),
            },
        );

        let history = memory_db
            .get_session_message(session_id)
            .unwrap_or_default();

        match send_message(&config.llm, &history) {
            Ok(response) => {
                process_message(
                    &memory_db,
                    &session_id,
                    ChatMessage {
                        role: "assistant".to_string(),
                        content: response.message.clone(),
                        timestamp: Utc::now().timestamp().to_string(),
                    },
                );

                execute_actions(&response.actions, &obsidian);
            }
            Err(e) => eprintln!("Ошибка: {}", e),
        }
    }
}

fn process_message(memory_db: &MemoryDb, session_id: &i64, msg: ChatMessage) {
    memory_db
        .save_message(*session_id, &msg)
        .expect("Error saving message");
    println!("{}: {}", msg.role, msg.content);
}

fn execute_actions(actions: &[LlmAction], obsidian: &ObsidianSkill) {
    for action in actions {
        match action.action_type.as_str() {
            "obsidian_note" => {
                println!("[ACTION] Obsidian: {:?}", action.title);
                let title = action.title.as_deref().unwrap_or("unknown");
                let content = action.content.as_deref().unwrap_or("");
                if let Err(e) = obsidian.write_note(title, content) {
                    eprintln!("[Obsidian] Ошибка: {}", e);
                }
            }
            "open" => {
                println!("[ACTION] Open: {:?}", action.path);
                // TODO: реализация
            }
            unknown => {
                eprintln!("[ACTION] Неизвестный тип: {}", unknown);
            }
        }
    }
}
