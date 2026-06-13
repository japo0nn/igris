use std::process::Command;
use std::thread;
use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use enigo::{
    Axis, Button, Coordinate,
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Mouse, Settings,
};
use screenshots::Screen;
use serde_json::json;

use crate::{
    configs::llm::LlmConfig,
    models::metadata::ModuleMetadata,
    skills::{MethodInfo, SkillError, SkillModule, SkillOutput},
};

#[derive(Debug, Clone)]
pub struct GuiSkill {
    pub metadata: ModuleMetadata,
    pub llm_config: LlmConfig,
}

impl SkillModule for GuiSkill {
    fn get_metadata(&self) -> &ModuleMetadata {
        &self.metadata
    }

    fn health_check(&self) -> bool {
        true
    }

    fn execute(&self, method: &str, args: &str) -> Result<SkillOutput, SkillError> {
        match method {
            "screenshot" => take_screenshot(),
            "analyze_screen" => self.analyze_screen(args),
            "click" => mouse_click(args),
            "move_mouse" => move_mouse_to(args),
            "type_text" => type_text(args),
            "scroll" => scroll(args),
            "key_press" => key_press(args),
            "open_url" => open_url(args),
            _ => Err(SkillError::InvalidArgs("Method does not exist".to_string())),
        }
    }

    fn available_methods(&self) -> Vec<MethodInfo> {
        vec![
            MethodInfo {
                method: "screenshot".to_string(),
                description: "Capture the current screen and save to /tmp/igris_screen.png. ALWAYS call this before analyze_screen or clicking. Requires Screen Recording permission on macOS.".to_string(),
                args_description: "No arguments required. Pass an empty string.".to_string(),
            },
            MethodInfo {
                method: "analyze_screen".to_string(),
                description: "Analyze the last screenshot using vision AI. Returns description of what is on screen including UI elements, text, and coordinates. Call screenshot first.".to_string(),
                args_description: "Question or instruction for the vision model. Example: 'Find the third video and give me its click coordinates' or 'What is on screen?'".to_string(),
            },
            MethodInfo {
                method: "click".to_string(),
                description: "Left-click at specific screen pixel coordinates. Use analyze_screen first to determine correct coordinates.".to_string(),
                args_description: "X and Y pixel coordinates separated by space. Example: 960 540".to_string(),
            },
            MethodInfo {
                method: "move_mouse".to_string(),
                description: "Move mouse cursor to specific coordinates without clicking.".to_string(),
                args_description: "X and Y pixel coordinates separated by space. Example: 960 540".to_string(),
            },
            MethodInfo {
                method: "type_text".to_string(),
                description: "Type text using the keyboard at the current focus position.".to_string(),
                args_description: "Text to type. Example: Hello World".to_string(),
            },
            MethodInfo {
                method: "scroll".to_string(),
                description: "Scroll the screen vertically at the current mouse position.".to_string(),
                args_description: "Direction and amount separated by space. Example: down 3 or up 5".to_string(),
            },
            MethodInfo {
                method: "key_press".to_string(),
                description: "Press keyboard keys or combinations. Supports: enter, escape, tab, space, backspace, delete, up, down, left, right, ctrl, alt, shift, cmd/meta/win, f1-f12 and single characters.".to_string(),
                args_description: "Key or combination using + separator. Example: enter, ctrl+t, cmd+c, f5".to_string(),
            },
            MethodInfo {
                method: "open_url".to_string(),
                description: "Open a URL in the default system browser. Cross-platform (macOS/Windows/Linux).".to_string(),
                args_description: "Full URL to open. Example: https://youtube.com".to_string(),
            },
        ]
    }
}

impl GuiSkill {
    fn analyze_screen(&self, question: &str) -> Result<SkillOutput, SkillError> {
        let path = "/tmp/igris_screen.png";

        let img_bytes = std::fs::read(path)
            .map_err(|_| SkillError::ExecutionFailed(
                "No screenshot found. Call 'screenshot' method first.".to_string()
            ))?;

        let img_b64 = BASE64.encode(&img_bytes);

        let api_key = self.llm_config.api_key.as_deref().unwrap_or("");
        let base_uri = self.llm_config.base_uri.trim_end_matches('/');
        let vision_model = &self.llm_config.vision_model;
        let url = format!("{}/v1/chat/completions", base_uri);

        let payload = json!({
            "model": vision_model,
            "max_tokens": 1024,
            "stream": false,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/png",
                            "data": img_b64
                        }
                    },
                    {
                        "type": "text",
                        "text": question
                    }
                ]
            }]
        });

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&payload)
            .timeout(Duration::from_secs(60))
            .send()
            .map_err(|e| SkillError::ExecutionFailed(format!("Vision API request failed: {}", e)))?;

        let json: serde_json::Value = response
            .json()
            .map_err(|e| SkillError::ExecutionFailed(format!("Failed to parse vision response: {}", e)))?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if content.is_empty() {
            return Err(SkillError::ExecutionFailed(
                format!("Vision model returned empty response. Full response: {}", json)
            ));
        }

        // Delete screenshot after analysis to avoid clutter
        let _ = std::fs::remove_file(path);

        Ok(SkillOutput::Text(content))
    }
}

fn take_screenshot() -> Result<SkillOutput, SkillError> {
    let path = "/tmp/igris_screen.png";
    let screens = Screen::all()
        .map_err(|e| SkillError::ExecutionFailed(format!("Failed to get screens: {}", e)))?;
    let screen = screens.into_iter().next()
        .ok_or_else(|| SkillError::ExecutionFailed("No screen found".to_string()))?;
    let image = screen.capture()
        .map_err(|e| SkillError::ExecutionFailed(format!("Failed to capture screen: {}", e)))?;
    let orig_width = image.width();
    let orig_height = image.height();

    // Resize to max 1280px on longest side for faster vision API response (cross-platform)
    const MAX_SIZE: u32 = 1280;
    let final_image = if orig_width > MAX_SIZE || orig_height > MAX_SIZE {
        let scale = MAX_SIZE as f32 / orig_width.max(orig_height) as f32;
        let new_w = (orig_width as f32 * scale) as u32;
        let new_h = (orig_height as f32 * scale) as u32;
        use screenshots::image::{DynamicImage, imageops::FilterType};
        DynamicImage::ImageRgba8(image).resize(new_w, new_h, FilterType::Triangle).to_rgba8()
    } else {
        image
    };

    final_image.save(path)
        .map_err(|e| SkillError::ExecutionFailed(format!("Failed to save screenshot: {}", e)))?;

    Ok(SkillOutput::Text(format!(
        "Screenshot saved to: {}. Original: {}x{} px (resized to max 1280px for performance). Now call analyze_screen to understand what is on screen.",
        path, orig_width, orig_height
    )))
}

fn mouse_click(args: &str) -> Result<SkillOutput, SkillError> {
    let (x, y) = parse_coordinates(args)?;
    let mut enigo = create_enigo()?;
    enigo.move_mouse(x, y, Coordinate::Abs)
        .map_err(|e| SkillError::ExecutionFailed(e.to_string()))?;
    thread::sleep(Duration::from_millis(50));
    enigo.button(Button::Left, Click)
        .map_err(|e| SkillError::ExecutionFailed(e.to_string()))?;
    Ok(SkillOutput::Text(format!("Clicked at ({}, {})", x, y)))
}

fn move_mouse_to(args: &str) -> Result<SkillOutput, SkillError> {
    let (x, y) = parse_coordinates(args)?;
    let mut enigo = create_enigo()?;
    enigo.move_mouse(x, y, Coordinate::Abs)
        .map_err(|e| SkillError::ExecutionFailed(e.to_string()))?;
    Ok(SkillOutput::Text(format!("Mouse moved to ({}, {})", x, y)))
}

fn type_text(args: &str) -> Result<SkillOutput, SkillError> {
    let mut enigo = create_enigo()?;
    enigo.text(args)
        .map_err(|e| SkillError::ExecutionFailed(e.to_string()))?;
    Ok(SkillOutput::Text(format!("Typed: {}", args)))
}

fn scroll(args: &str) -> Result<SkillOutput, SkillError> {
    let parts: Vec<&str> = args.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return Err(SkillError::InvalidArgs(
            "Expected: up/down amount. Example: down 3".to_string(),
        ));
    }
    let direction = parts[0].to_lowercase();
    let amount: i32 = parts[1]
        .parse()
        .map_err(|_| SkillError::InvalidArgs("Amount must be an integer".to_string()))?;
    let scroll_amount = if direction == "up" { -amount } else { amount };
    let mut enigo = create_enigo()?;
    enigo.scroll(scroll_amount, Axis::Vertical)
        .map_err(|e| SkillError::ExecutionFailed(e.to_string()))?;
    Ok(SkillOutput::Text(format!("Scrolled {} by {}", direction, amount)))
}

fn key_press(args: &str) -> Result<SkillOutput, SkillError> {
    let mut enigo = create_enigo()?;
    let parts: Vec<&str> = args.trim().split('+').map(|s| s.trim()).collect();
    let mut modifiers: Vec<Key> = Vec::new();
    let mut main_key: Option<Key> = None;
    for part in &parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers.push(Key::Control),
            "alt" => modifiers.push(Key::Alt),
            "shift" => modifiers.push(Key::Shift),
            "cmd" | "meta" | "win" | "super" => modifiers.push(Key::Meta),
            "enter" | "return" => main_key = Some(Key::Return),
            "escape" | "esc" => main_key = Some(Key::Escape),
            "tab" => main_key = Some(Key::Tab),
            "space" => main_key = Some(Key::Space),
            "backspace" => main_key = Some(Key::Backspace),
            "delete" | "del" => main_key = Some(Key::Delete),
            "up" => main_key = Some(Key::UpArrow),
            "down" => main_key = Some(Key::DownArrow),
            "left" => main_key = Some(Key::LeftArrow),
            "right" => main_key = Some(Key::RightArrow),
            "f1" => main_key = Some(Key::F1),
            "f2" => main_key = Some(Key::F2),
            "f3" => main_key = Some(Key::F3),
            "f4" => main_key = Some(Key::F4),
            "f5" => main_key = Some(Key::F5),
            "f6" => main_key = Some(Key::F6),
            "f7" => main_key = Some(Key::F7),
            "f8" => main_key = Some(Key::F8),
            "f9" => main_key = Some(Key::F9),
            "f10" => main_key = Some(Key::F10),
            "f11" => main_key = Some(Key::F11),
            "f12" => main_key = Some(Key::F12),
            s if s.len() == 1 => {
                main_key = Some(Key::Unicode(s.chars().next().unwrap()));
            }
            _ => {}
        }
    }
    for m in &modifiers {
        enigo.key(*m, Press)
            .map_err(|e| SkillError::ExecutionFailed(e.to_string()))?;
    }
    if let Some(k) = main_key {
        enigo.key(k, Click)
            .map_err(|e| SkillError::ExecutionFailed(e.to_string()))?;
    } else if modifiers.is_empty() {
        enigo.text(args)
            .map_err(|e| SkillError::ExecutionFailed(e.to_string()))?;
    }
    for m in modifiers.iter().rev() {
        enigo.key(*m, Release)
            .map_err(|e| SkillError::ExecutionFailed(e.to_string()))?;
    }
    Ok(SkillOutput::Text(format!("Key pressed: {}", args)))
}

fn open_url(args: &str) -> Result<SkillOutput, SkillError> {
    open_in_browser(args.trim())?;
    Ok(SkillOutput::Text(format!("Opened: {}", args.trim())))
}

#[cfg(target_os = "macos")]
fn open_in_browser(url: &str) -> Result<(), SkillError> {
    Command::new("open").arg(url).status()
        .map_err(|e| SkillError::ExecutionFailed(format!("Failed to open URL: {}", e)))
        .map(|_| ())
}

#[cfg(target_os = "windows")]
fn open_in_browser(url: &str) -> Result<(), SkillError> {
    Command::new("cmd").args(["/C", "start", "", url]).status()
        .map_err(|e| SkillError::ExecutionFailed(format!("Failed to open URL: {}", e)))
        .map(|_| ())
}

#[cfg(target_os = "linux")]
fn open_in_browser(url: &str) -> Result<(), SkillError> {
    Command::new("xdg-open").arg(url).status()
        .map_err(|e| SkillError::ExecutionFailed(format!("Failed to open URL: {}", e)))
        .map(|_| ())
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn open_in_browser(_url: &str) -> Result<(), SkillError> {
    Err(SkillError::ExecutionFailed("Unsupported OS".to_string()))
}

fn parse_coordinates(args: &str) -> Result<(i32, i32), SkillError> {
    let parts: Vec<&str> = args.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return Err(SkillError::InvalidArgs(
            "Expected: x y coordinates. Example: 960 540".to_string(),
        ));
    }
    let x: i32 = parts[0].parse()
        .map_err(|_| SkillError::InvalidArgs("X must be an integer".to_string()))?;
    let y: i32 = parts[1].parse()
        .map_err(|_| SkillError::InvalidArgs("Y must be an integer".to_string()))?;
    Ok((x, y))
}

fn create_enigo() -> Result<Enigo, SkillError> {
    Enigo::new(&Settings::default()).map_err(|e| {
        SkillError::ExecutionFailed(format!(
            "Failed to initialize input controller (check Accessibility permissions): {}",
            e
        ))
    })
}
