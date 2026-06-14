use reqwest::blocking::Client;
use scraper::{Html, Selector};

use crate::{
    models::metadata::ModuleMetadata,
    skills::{MethodInfo, SkillError, SkillModule, SkillOutput},
};

pub struct WebSearchSkill {
    pub metadata: ModuleMetadata,
}

impl WebSearchSkill {
    pub fn new() -> Self {
        WebSearchSkill {
            metadata: ModuleMetadata {
                name: "WebSearchSkill".to_string(),
                version: "0.1.0".to_string(),
                _type: crate::models::metadata::ModuleType::Persistent,
                description: "Search the web and read webpage content using DuckDuckGo and HTML parsing.".to_string(),
                author: Some("IGRIS".to_string()),
            },
        }
    }
}

impl WebSearchSkill {
    fn search_duckduckgo(&self, query: &str) -> Result<String, SkillError> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Igris/0.1")
            .build()
            .map_err(|e| SkillError::ExecutionFailed(format!("Failed to build client: {}", e)))?;

        let url = format!("https://lite.duckduckgo.com/lite/?q={}", urlencoding(query));
        let resp = client
            .get(&url)
            .send()
            .map_err(|e| SkillError::ExecutionFailed(format!("Request failed: {}", e)))?
            .text()
            .map_err(|e| SkillError::ExecutionFailed(format!("Read body failed: {}", e)))?;

        let document = Html::parse_document(&resp);
        let result_selector =
            Selector::parse(".result__title a, .result__snippet").map_err(|e| {
                SkillError::ExecutionFailed(format!("Selector error: {}", e))
            })?;

        let mut results = Vec::new();
        let mut current_title = String::new();

        for element in document.select(&result_selector) {
            let text = element.text().collect::<String>().trim().to_string();
            if element.value().has_class("result__title", scraper::CaseSensitivity::CaseSensitive) ||
               element.value().has_class("result__title", scraper::CaseSensitivity::AsciiCaseInsensitive) {
                if let Some(href) = element.value().attr("href") {
                    current_title = format!("[{}]({})", text, href);
                } else {
                    current_title = text;
                }
            } else {
                if !current_title.is_empty() {
                    results.push(format!("{} — {}", current_title, text));
                    current_title.clear();
                }
            }
        }

        if results.is_empty() {
            Ok("No results found.".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }

    fn read_webpage(&self, url: &str) -> Result<String, SkillError> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Igris/0.1")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| SkillError::ExecutionFailed(format!("Client build: {}", e)))?;

        let resp = client
            .get(url)
            .send()
            .map_err(|e| SkillError::ExecutionFailed(format!("Request failed: {}", e)))?
            .text()
            .map_err(|e| SkillError::ExecutionFailed(format!("Read body: {}", e)))?;

        let document = Html::parse_document(&resp);
        let body_selector = Selector::parse("body").unwrap();
        let text_selector = Selector::parse("p, h1, h2, h3, h4, h5, h6, li, pre, code, blockquote").unwrap();

        let mut content = Vec::new();

        if let Some(body) = document.select(&body_selector).next() {
            for element in body.select(&text_selector) {
                let text = element.text().collect::<String>().trim().to_string();
                if !text.is_empty() {
                    let tag_name = element.value().name();
                    let prefix = match tag_name {
                        "h1" => "# ",
                        "h2" => "## ",
                        "h3" => "### ",
                        "h4" => "#### ",
                        "h5" => "##### ",
                        "h6" => "###### ",
                        "pre" | "code" => "  `",
                        "li" => "- ",
                        _ => "",
                    };
                    let suffix = if tag_name == "pre" || tag_name == "code" { "`" } else { "" };
                    content.push(format!("{}{}{}", prefix, text, suffix));
                }
            }
        }

        let output = content.join("\n");
        if output.len() > 5000 {
            Ok(format!("{}...\n[Truncated to 5000 chars]", &output[..output.floor_char_boundary(5000)]))
        } else if output.is_empty() {
            Ok("Page appears to be empty or unreadable.".to_string())
        } else {
            Ok(output)
        }
    }
}

impl SkillModule for WebSearchSkill {
    fn get_metadata(&self) -> &ModuleMetadata {
        &self.metadata
    }

    fn health_check(&self) -> bool {
        true
    }

    fn execute(&self, method: &str, args: &str) -> Result<SkillOutput, SkillError> {
        match method {
            "search_web" => {
                if args.trim().is_empty() {
                    return Err(SkillError::InvalidArgs("Query is required".to_string()));
                }
                let result = self.search_duckduckgo(args.trim())?;
                Ok(SkillOutput::Text(result))
            }
            "read_page" => {
                if args.trim().is_empty() {
                    return Err(SkillError::InvalidArgs("URL is required".to_string()));
                }
                let result = self.read_webpage(args.trim())?;
                Ok(SkillOutput::Text(result))
            }
            _ => Err(SkillError::NotFound(format!("Method '{}' not found", method))),
        }
    }

    fn available_methods(&self) -> Vec<MethodInfo> {
        vec![
            MethodInfo {
                method: "search_web".to_string(),
                description: "Search the internet using DuckDuckGo. Returns up to 10 results with titles, snippets, and links.".to_string(),
                args_description: "Search query. Example: 'rust programming tutorial'".to_string(),
            },
            MethodInfo {
                method: "read_page".to_string(),
                description: "Fetch and extract readable content from any URL. Returns text from p, headings, lists, and code blocks.".to_string(),
                args_description: "Full URL to read. Example: 'https://www.rust-lang.org/learn'".to_string(),
            },
        ]
    }
}

fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => '+'.to_string(),
            other => format!(
                "%{:02X}",
                other as u8
            ),
        })
        .collect()
}
