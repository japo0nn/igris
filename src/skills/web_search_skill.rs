use reqwest::blocking::Client as BlockingClient;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, USER_AGENT};
use scraper::{Html, Selector};
use std::time::Duration;

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
                version: "0.2.0".to_string(),
                _type: crate::models::metadata::ModuleType::Persistent,
                description: "Search the web using DuckDuckGo (primary) and SearXNG as fallback."
                    .to_string(),
                author: Some("IGRIS".to_string()),
            },
        }
    }

    fn build_client() -> Result<BlockingClient, SkillError> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".parse().unwrap());
        headers.insert(
            ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
                .parse()
                .unwrap(),
        );
        headers.insert(ACCEPT_LANGUAGE, "en-US,en;q=0.9".parse().unwrap());

        BlockingClient::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| SkillError::ExecutionFailed(format!("Failed to build client: {}", e)))
    }

    fn search_duckduckgo(&self, query: &str) -> Result<Option<String>, SkillError> {
        let client = Self::build_client()?;
        let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding(query));

        let resp = client
            .get(&url)
            .send()
            .map_err(|e| SkillError::ExecutionFailed(format!("Request failed: {}", e)))?;

        let body = resp
            .text()
            .map_err(|e| SkillError::ExecutionFailed(format!("Read body: {}", e)))?;

        // Check for captcha
        if body.contains("challenge-form")
            || body.contains("anomaly-modal")
            || body.contains("Please complete the following challenge")
        {
            return Ok(None); // captcha detected
        }

        let document = Html::parse_document(&body);
        let result_selector = Selector::parse(".result__body")
            .map_err(|_| SkillError::ExecutionFailed("Selector parse error".to_string()))?;

        let mut results = Vec::new();

        for element in document.select(&result_selector) {
            let title_sel = Selector::parse(".result__title a").unwrap();
            let snippet_sel = Selector::parse(".result__snippet").unwrap();

            let title = element
                .select(&title_sel)
                .next()
                .and_then(|a| a.text().collect::<String>().into())
                .unwrap_or_default()
                .trim()
                .to_string();

            let url = element
                .select(&title_sel)
                .next()
                .and_then(|a| a.value().attr("href"))
                .unwrap_or("")
                .to_string();

            let snippet = element
                .select(&snippet_sel)
                .next()
                .map(|s| s.text().collect::<String>())
                .unwrap_or_default()
                .trim()
                .to_string();

            if !title.is_empty() {
                let clean_url = url
                    .split("uddg=")
                    .nth(1)
                    .and_then(|u| u.split('&').next())
                    .and_then(|u| urlencoding_decode(u))
                    .unwrap_or(url)
                    .to_string();

                if snippet.is_empty() {
                    results.push(format!("[{}]({})", title, clean_url));
                } else {
                    results.push(format!("[{}]({}) - {}", title, clean_url, snippet));
                }
            }

            if results.len() >= 8 {
                break;
            }
        }

        if results.is_empty() {
            Ok(None)
        } else {
            Ok(Some(results.join("\n")))
        }
    }

    fn search_searxng(&self, query: &str) -> Result<String, SkillError> {
        let instances = vec![
            "https://searx.be",
            "https://search.sapti.me",
            "https://search.studentb-tech.space",
            "https://priv.au",
            "https://s.mble.dk",
        ];

        let client = BlockingClient::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| SkillError::ExecutionFailed(format!("Client build: {}", e)))?;

        let mut last_error = String::new();

        for instance in &instances {
            let url = format!(
                "{}/search?q={}&format=json&language=en&categories=general&pageno=1",
                instance,
                urlencoding(query)
            );

            match client.get(&url).header(USER_AGENT, "Mozilla/5.0").send() {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        last_error = format!("{} returned status {}", instance, resp.status());
                        continue;
                    }

                    match resp.json::<serde_json::Value>() {
                        Ok(json) => {
                            let results = json["results"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .take(8)
                                        .filter_map(|r| {
                                            let title =
                                                r["title"].as_str().unwrap_or("").to_string();
                                            let url = r["url"].as_str().unwrap_or("").to_string();
                                            let content =
                                                r["content"].as_str().unwrap_or("").to_string();
                                            if title.is_empty() && url.is_empty() {
                                                None
                                            } else if content.is_empty() {
                                                Some(format!("[{}]({})", title, url))
                                            } else {
                                                Some(format!("[{}]({}) - {}", title, url, content))
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();

                            if !results.is_empty() {
                                return Ok(results.join("\n"));
                            }
                            last_error = format!("{} returned empty results", instance);
                        }
                        Err(e) => {
                            last_error = format!("{} JSON parse: {}", instance, e);
                        }
                    }
                }
                Err(e) => {
                    last_error = format!("{} connection: {}", instance, e);
                }
            }
        }

        Err(SkillError::ExecutionFailed(format!(
            "All SearXNG instances failed: {}",
            last_error
        )))
    }

    fn read_webpage(&self, url: &str) -> Result<String, SkillError> {
        let client = Self::build_client()?;

        let resp = client
            .get(url)
            .send()
            .map_err(|e| SkillError::ExecutionFailed(format!("Request failed: {}", e)))?
            .text()
            .map_err(|e| SkillError::ExecutionFailed(format!("Read body: {}", e)))?;

        let document = Html::parse_document(&resp);
        let body_selector = Selector::parse("body").unwrap();
        let text_selector =
            Selector::parse("p, h1, h2, h3, h4, h5, h6, li, pre, code, blockquote").unwrap();

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
                    let suffix = if tag_name == "pre" || tag_name == "code" {
                        "`"
                    } else {
                        ""
                    };
                    content.push(format!("{}{}{}", prefix, text, suffix));
                }
            }
        }

        let output = content.join("\n");
        if output.len() > 5000 {
            Ok(format!(
                "{}...\n[Truncated to 5000 chars]",
                &output[..output.floor_char_boundary(5000)]
            ))
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
                let query = args.trim();

                // Try DuckDuckGo first
                match self.search_duckduckgo(query) {
                    Ok(Some(results)) => return Ok(SkillOutput::Text(results)),
                    Ok(None) => { /* captcha or no results, fall through */ }
                    Err(_) => { /* fall through to SearXNG */ }
                }

                // Fallback to SearXNG
                let result = self.search_searxng(query)?;
                Ok(SkillOutput::Text(result))
            }
            "read_page" => {
                if args.trim().is_empty() {
                    return Err(SkillError::InvalidArgs("URL is required".to_string()));
                }
                let result = self.read_webpage(args.trim())?;
                Ok(SkillOutput::Text(result))
            }
            _ => Err(SkillError::NotFound(format!(
                "Method '{}' not found",
                method
            ))),
        }
    }

    fn available_methods(&self) -> Vec<MethodInfo> {
        vec![
            MethodInfo {
                method: "search_web".to_string(),
                description: "Search the internet using DuckDuckGo with SearXNG fallback. Returns up to 8 results.".to_string(),
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
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => result.push_str(&format!("%{:02X}", byte)),
        }
    }
    result
}

fn urlencoding_decode(s: &str) -> Option<String> {
    let mut bytes: Vec<u8> = Vec::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    bytes.push(byte);
                    continue;
                }
            }
        }
        let mut buf = [0u8; 4];
        bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}
