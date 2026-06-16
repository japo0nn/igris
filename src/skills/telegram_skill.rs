use std::sync::{Arc, Mutex};

use ferogram::{Client, InputMessage, SignInError, TransportKind};
use tokio::runtime::Runtime;

use crate::{
    configs::llm::TelegramSecrets,
    models::metadata::ModuleMetadata,
    skills::{MethodInfo, SkillError, SkillModule, SkillOutput},
};

struct TgState {
    client: Option<Client>,
    shutdown: Option<Box<dyn std::any::Any + Send>>,
    login_token: Option<ferogram::LoginToken>,
}

pub struct TelegramSkill {
    pub metadata: ModuleMetadata,
    secrets: Option<TelegramSecrets>,
    runtime: Arc<Runtime>,
    state: Arc<Mutex<TgState>>,
}

impl TelegramSkill {
    pub fn new(secrets: Option<TelegramSecrets>) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
            .expect("Failed to build Telegram tokio runtime");

        TelegramSkill {
            metadata: ModuleMetadata {
                name: "TelegramSkill".to_string(),
                version: "0.1.0".to_string(),
                _type: crate::models::metadata::ModuleType::Persistent,
                description:
                    "Native Telegram MTProto user-account client (ferogram): dialogs, history, send."
                        .to_string(),
                author: Some("IGRIS".to_string()),
            },
            secrets,
            runtime: Arc::new(runtime),
            state: Arc::new(Mutex::new(TgState {
                client: None,
                shutdown: None,
                login_token: None,
            })),
        }
    }

    fn creds(&self) -> Result<&TelegramSecrets, SkillError> {
        match &self.secrets {
            Some(s) if s.is_valid() => Ok(s),
            Some(_) => Err(SkillError::ExecutionFailed(
                "Telegram credentials present but invalid.".to_string(),
            )),
            None => Err(SkillError::ExecutionFailed(
                "No [telegram] credentials in secrets.toml.".to_string(),
            )),
        }
    }

    fn ensure_connected(&self, st: &mut TgState) -> Result<(), SkillError> {
        if st.client.is_some() {
            return Ok(());
        }
        let creds = self.creds()?.clone();
        let rt = &self.runtime;
        let result: Result<(Client, Box<dyn std::any::Any + Send>), String> =
            std::thread::scope(|s| {
                s.spawn(|| {
                    rt.block_on(async {
                        let (client, shutdown) = Client::builder()
                            .api_id(creds.api_id)
                            .api_hash(&creds.api_hash)
                            .session(&creds.session_path)
                            .transport(TransportKind::Abridged)
                            .connect()
                            .await
                            .map_err(|e| format!("connect failed: {}", e))?;
                        let boxed: Box<dyn std::any::Any + Send> = Box::new(shutdown);
                        Ok::<_, String>((client, boxed))
                    })
                })
                .join()
                .unwrap()
            });
        let (client, shutdown) = result.map_err(SkillError::ExecutionFailed)?;
        st.client = Some(client);
        st.shutdown = Some(shutdown);
        Ok(())
    }

    fn status(&self) -> Result<SkillOutput, SkillError> {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        self.ensure_connected(&mut st)?;
        let rt = &self.runtime;
        let client = st.client.as_ref().unwrap();
        let out: Result<String, String> = std::thread::scope(|s| {
            s.spawn(|| {
                rt.block_on(async {
                    let authed = client
                        .is_authorized()
                        .await
                        .map_err(|e| format!("is_authorized failed: {}", e))?;
                    if !authed {
                        return Ok::<_, String>("NOT_AUTHORIZED: call login".to_string());
                    }
                    let me = client
                        .get_me()
                        .await
                        .map_err(|e| format!("get_me failed: {}", e))?;
                    Ok(format!(
                        "AUTHORIZED as {} {} (id={})",
                        me.first_name.as_deref().unwrap_or(""),
                        me.last_name.as_deref().unwrap_or(""),
                        me.id
                    ))
                })
            })
            .join()
            .unwrap()
        });
        Ok(SkillOutput::Text(out.map_err(SkillError::ExecutionFailed)?))
    }

    fn login(&self) -> Result<SkillOutput, SkillError> {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        self.ensure_connected(&mut st)?;
        let creds = self.creds()?.clone();
        let rt = &self.runtime;
        let client = st.client.as_ref().unwrap();
        let res: Result<(String, Option<ferogram::LoginToken>), String> = std::thread::scope(|s| {
            s.spawn(|| {
                rt.block_on(async {
                    if client
                        .is_authorized()
                        .await
                        .map_err(|e| format!("is_authorized failed: {}", e))?
                    {
                        return Ok::<_, String>(("Already authorized.".to_string(), None));
                    }
                    let token = client
                        .request_login_code(&creds.phone_number)
                        .await
                        .map_err(|e| format!("request_login_code failed: {}", e))?;
                    Ok((
                        "Code sent to your Telegram. Call submit_code with the code.".to_string(),
                        Some(token),
                    ))
                })
            })
            .join()
            .unwrap()
        });
        let (msg, token) = res.map_err(SkillError::ExecutionFailed)?;
        if token.is_some() {
            st.login_token = token;
        }
        Ok(SkillOutput::Text(msg))
    }

    fn submit_code(&self, code: &str) -> Result<SkillOutput, SkillError> {
        let code = code.trim().to_string();
        if code.is_empty() {
            return Err(SkillError::InvalidArgs(
                "Provide the login code.".to_string(),
            ));
        }
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        self.ensure_connected(&mut st)?;
        let token = st.login_token.take().ok_or_else(|| {
            SkillError::ExecutionFailed("No login in progress. Call login first.".to_string())
        })?;
        let rt = &self.runtime;
        let client = st.client.as_ref().unwrap();
        let res: Result<String, String> = std::thread::scope(|s| {
            s.spawn(|| {
                rt.block_on(async {
                    match client.sign_in(&token, &code).await {
                        Ok(name) => {
                            client
                                .save_session()
                                .await
                                .map_err(|e| format!("save_session failed: {}", e))?;
                            Ok::<_, String>(format!("Signed in as {}. Session saved.", name))
                        }
                        Err(SignInError::PasswordRequired(_)) => {
                            Err("2FA password required - not supported in this build yet."
                                .to_string())
                        }
                        Err(SignInError::SignUpRequired) => {
                            Err("Phone not registered on Telegram.".to_string())
                        }
                        Err(e) => Err(format!("sign_in failed: {}", e)),
                    }
                })
            })
            .join()
            .unwrap()
        });
        Ok(SkillOutput::Text(res.map_err(SkillError::ExecutionFailed)?))
    }

    fn list_dialogs(&self, args: &str) -> Result<SkillOutput, SkillError> {
        let limit: i32 = args.trim().parse().unwrap_or(20);
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        self.ensure_connected(&mut st)?;
        let rt = &self.runtime;
        let client = st.client.as_ref().unwrap();
        let res: Result<String, String> = std::thread::scope(|s| {
            s.spawn(|| {
                rt.block_on(async {
                    let dialogs = client
                        .get_dialogs(limit)
                        .await
                        .map_err(|e| format!("get_dialogs failed: {}", e))?;
                    let mut lines = Vec::new();
                    for (i, d) in dialogs.iter().enumerate() {
                        let cid = d
                            .peer()
                            .map(|p| match p {
                                ferogram::tl::enums::Peer::User(u) => u.user_id,
                                ferogram::tl::enums::Peer::Chat(c) => -c.chat_id,
                                ferogram::tl::enums::Peer::Channel(c) => {
                                    -1_000_000_000_000i64 - c.channel_id
                                }
                            })
                            .unwrap_or(0);
                        lines.push(format!(
                            "{}. {} [id: {}] (unread: {})",
                            i + 1,
                            d.title(),
                            cid,
                            d.unread_count()
                        ));
                    }
                    if lines.is_empty() {
                        Ok::<_, String>("(no dialogs)".to_string())
                    } else {
                        Ok(lines.join("\n"))
                    }
                })
            })
            .join()
            .unwrap()
        });
        Ok(SkillOutput::Text(res.map_err(SkillError::ExecutionFailed)?))
    }

    fn read_chat(&self, args: &str) -> Result<SkillOutput, SkillError> {
        let parts: Vec<&str> = args.splitn(2, '|').map(|s| s.trim()).collect();
        if parts.is_empty() || parts[0].is_empty() {
            return Err(SkillError::InvalidArgs(
                "Expected: peer | limit".to_string(),
            ));
        }
        let peer = parts[0].to_string();
        let limit: i32 = parts.get(1).and_then(|x| x.parse().ok()).unwrap_or(20);
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        self.ensure_connected(&mut st)?;
        let rt = &self.runtime;
        let client = st.client.as_ref().unwrap();
        let res: Result<String, String> = std::thread::scope(|s| {
            s.spawn(|| {
                rt.block_on(async {
                    let messages = client
                        .get_message_history(peer.as_str(), limit, 0)
                        .await
                        .map_err(|e| format!("get_message_history failed: {}", e))?;
                    let mut lines = Vec::new();
                    for m in &messages {
                        let ts = m
                            .date_utc()
                            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_default();
                        let body = m.text().unwrap_or("").trim().to_string();
                        lines.push(format!("[{}] {} {}", m.id(), ts, body));
                    }
                    if lines.is_empty() {
                        Ok::<_, String>("(no messages)".to_string())
                    } else {
                        Ok(lines.join("\n"))
                    }
                })
            })
            .join()
            .unwrap()
        });
        Ok(SkillOutput::Text(res.map_err(SkillError::ExecutionFailed)?))
    }

    fn send_message(&self, args: &str) -> Result<SkillOutput, SkillError> {
        let parts: Vec<&str> = args.splitn(2, '|').map(|s| s.trim()).collect();
        if parts.len() < 2 || parts[0].is_empty() {
            return Err(SkillError::InvalidArgs("Expected: peer | text".to_string()));
        }
        let peer = parts[0].to_string();
        let text = parts[1].to_string();
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        self.ensure_connected(&mut st)?;
        let rt = &self.runtime;
        let client = st.client.as_ref().unwrap();
        let res: Result<String, String> = std::thread::scope(|s| {
            s.spawn(|| {
                rt.block_on(async {
                    client
                        .send_message(peer.as_str(), InputMessage::text(text.as_str()))
                        .await
                        .map_err(|e| format!("send_message failed: {}", e))?;
                    Ok::<_, String>("Message sent.".to_string())
                })
            })
            .join()
            .unwrap()
        });
        Ok(SkillOutput::Text(res.map_err(SkillError::ExecutionFailed)?))
    }
}

impl SkillModule for TelegramSkill {
    fn get_metadata(&self) -> &ModuleMetadata {
        &self.metadata
    }

    fn health_check(&self) -> bool {
        self.secrets.as_ref().map(|s| s.is_valid()).unwrap_or(false)
    }

    fn execute(&self, method: &str, args: &str) -> Result<SkillOutput, SkillError> {
        match method {
            "status" => self.status(),
            "login" => self.login(),
            "submit_code" => self.submit_code(args),
            "list_dialogs" => self.list_dialogs(args),
            "read_chat" => self.read_chat(args),
            "send_message" => self.send_message(args),
            _ => Err(SkillError::InvalidArgs("Method does not exist".to_string())),
        }
    }

    fn available_methods(&self) -> Vec<MethodInfo> {
        vec![
            MethodInfo {
                method: "status".to_string(),
                description: "Check Telegram connection/auth status and current account."
                    .to_string(),
                args_description: "No arguments. Pass an empty string.".to_string(),
            },
            MethodInfo {
                method: "login".to_string(),
                description:
                    "Start login: requests an SMS/app login code for the configured phone."
                        .to_string(),
                args_description: "No arguments. Pass an empty string.".to_string(),
            },
            MethodInfo {
                method: "submit_code".to_string(),
                description: "Submit the login code received in Telegram to finish sign-in."
                    .to_string(),
                args_description: "The login code. Example: 12345".to_string(),
            },
            MethodInfo {
                method: "list_dialogs".to_string(),
                description: "List recent chats/channels/groups.".to_string(),
                args_description: "Optional limit number. Example: 20".to_string(),
            },
            MethodInfo {
                method: "read_chat".to_string(),
                description: "Read message history from a peer (me/username/id).".to_string(),
                args_description: "peer | limit. Example: me | 20".to_string(),
            },
            MethodInfo {
                method: "send_message".to_string(),
                description: "Send a text message to a peer (me/username/id).".to_string(),
                args_description: "peer | text. Example: me | Hello".to_string(),
            },
        ]
    }
}
