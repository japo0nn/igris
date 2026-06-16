# IGRIS
## Intelligent General Runtime & Integrated System

[![Rust](https://img.shields.io/badge/Rust-2024%20Edition-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![Status](https://img.shields.io/badge/Status-Active%20Development-brightgreen?style=flat-square)]()

> A modular, context-aware personal PC assistant written in Rust.
> Persistent memory meets a flexible skill architecture for seamless multi-turn interactions via a JSON-based protocol.

---

## What is IGRIS?

IGRIS is an intelligent agent that:

- Remembers everything - SQLite-powered memory with 9 retrieval methods
- Sees your screen - Vision AI + GUI automation (screenshot, click, keyboard)
- Searches the web - DuckDuckGo with SearXNG fallback
- Speaks and listens - TTS output and continuous voice input (Whisper)
- Executes smartly - Multi-step actions with streaming context
- Stays in sync - Automatic session restoration on restart
- Speaks JSON - Pure JSON-based communication protocol
- Extends easily - Modular skill architecture
- Has a Web UI - React frontend + REST API server mode

---

## Skills

IGRIS ships with six built-in skills:

### 1. Memory - Persistent Knowledge Base

All conversations persist in SQLite. Retrieve with precision:

| Method | Description | Arguments |
|--------|-------------|-----------|
| by-topics | Retrieve messages by topic tags | Space-separated topics |
| get-sessions | List all sessions | (empty) |
| get-messages-by-time-range | Messages within a time window | start|end |
| get-messages-paginated | Browse page-by-page | page size |
| get-messages-by-session | All messages from one session | Session UUID |
| get-topics | Discover all topic tags | (empty) |
| search-messages | Keyword search (limited to 50) | Keyword |
| get-message-by-id | Fetch one message | Message UUID |
| get-sessions-by-date | Sessions within a date range | start|end |

### 2. ShellExecutor - Command Runner

Runs shell commands with exit-code validation. PowerShell on Windows, sh on Unix.

### 3. GuiSkill - Screen Automation

| Method | Description | Arguments |
|--------|-------------|-----------|
| screenshot | Capture the screen | (empty) |
| analyze_screen | Analyze with vision AI | Question |
| click | Left-click at coordinates | X Y |
| move_mouse | Move cursor | X Y |
| type_text | Type text | Text |
| scroll | Scroll vertically | up/down N |
| key_press | Press keys/combos | enter, ctrl+c, ... |
| open_url | Open URL in browser | Full URL |

Screenshots are saved to the OS temp directory (cross-platform via std::env::temp_dir()).

### 4. WebSearchSkill - Internet Access

| Method | Description | Arguments |
|--------|-------------|-----------|
| search_web | DuckDuckGo + SearXNG fallback | Query |
| read_page | Extract readable text from a URL | Full URL |

### 5. UserProfileSkill - Persistent Preferences

| Method | Description | Arguments |
|--------|-------------|-----------|
| get-profile | Return the full profile as JSON | (empty) |
| update-preference | Save a key-value preference | key|value |
| add-topic | Add a topic of interest | Topic name |

Profile is stored at ~/.igris/user_profile.json and persists across runs.

### 6. Voice - Text-to-Speech

| Method | Description | Arguments |
|--------|-------------|-----------|
| speak | Speak text aloud | Text |

Uses macOS say, Linux espeak/spd-say, or Windows PowerShell SAPI.

---

## Usage Modes

```bash
# Interactive terminal mode (default)
igris

# Single message - process and exit
igris --message "What files are on my Desktop?"
igris -m "Show me the weather"

# REST API server mode (http://localhost:3001)
igris --server
igris -s

# Continuous voice mode (microphone -> Whisper -> agent -> TTS)
igris --voice
igris -v

# Full Web UI (backend + React frontend)
./start-ui.sh

# Help
igris --help
```

Interactive mode supports slash commands: /help, /clear, /history, /edit, /exit.

---

## Architecture

```
igris/
|-- src/
|   |-- core/
|   |   |-- agent.rs       # Agent loop orchestration
|   |   |-- chat.rs        # Interactive REPL (rustyline)
|   |   |-- llm.rs         # LLM calls (ask_llm, generate_topics)
|   |   |-- task.rs        # Task object building + topic saving
|   |   |-- markdown.rs    # Terminal markdown renderer
|   |   |-- spinner.rs     # Async progress spinner
|   |   |-- utils.rs       # Shared helpers (parse_db_timestamp, speak_text)
|   |   +-- mod.rs         # CoreContext
|   |-- skills/
|   |   |-- memory_skill.rs        # SQLite memory (9 methods)
|   |   |-- shell_executor.rs      # Shell commands
|   |   |-- gui_skill.rs           # GUI + vision AI
|   |   |-- web_search_skill.rs    # DuckDuckGo / SearXNG
|   |   |-- user_profile_skill.rs  # Persistent profile
|   |   |-- voice_skill.rs         # TTS
|   |   +-- mod.rs                 # SkillModule trait + helpers
|   |-- voice/
|   |   |-- continuous.rs   # Mic capture + Whisper transcription
|   |   +-- mod.rs
|   |-- memory/mod.rs      # Session & Message models
|   |-- models/            # ActionResponse, metadata, etc.
|   |-- configs/llm.rs     # AppConfig / SecretsConfig
|   |-- api.rs             # REST API (axum)
|   |-- db.rs              # SQLite operations
|   |-- error.rs           # IgrisError
|   |-- registry.rs        # Skill registration
|   |-- supervisor.rs      # Lifecycle logging + backups
|   +-- main.rs            # Entry point + CLI flags
|-- ui/                   # React frontend (Vite)
|-- start-ui.sh           # Launch backend + frontend
|-- config.toml           # Main configuration
|-- secrets.toml          # API keys (git-ignored)
|-- Cargo.toml
+-- README.md
```

---

## Configuration

IGRIS reads config.toml (committed) and secrets.toml (git-ignored) from the project root.

> IGRIS talks to an OpenAI-compatible endpoint via base_uri. Point it at your local LLM proxy or any compatible gateway.

### secrets.toml

Create this file manually in the project root. Never commit it.

```toml
[llm]
api_key = "your-api-key-here"

# Optional - only needed for --voice mode
[voice]
groq_api_key = "your-groq-key-here"
```

### config.toml reference

```toml
[memory]
db_path = "./igris.db"

[llm]
model = "cc/claude-opus-4-8"
vision_model = "cc/claude-sonnet-4-6"
base_uri = "http://localhost:20128/api"
system_prompt = "..."            # IGRIS system instructions
context_token_limit = 128000    # Tokens before trimming history
retention_days = 7              # Days to keep when trimming
retry_max_retries = 3           # LLM retry attempts
retry_initial_delay_ms = 1000   # Backoff base delay

[topic_llm]
model = "kr/claude-haiku-4.5"
vision_model = "cc/claude-sonnet-4-6"
system_prompt = "..."            # Topic extraction prompt

[execution]
iteration_limit = 10            # Max agent iterations
fix_iteration_limit = 5         # Max self-correction attempts
```

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| tokio | Async runtime |
| serde / serde_json | Serialization |
| toml | Config parsing |
| rusqlite (bundled) | SQLite driver |
| uuid | Unique identifiers |
| chrono | DateTime handling |
| reqwest | HTTP client (LLM, web) |
| scraper | HTML parsing for web search |
| axum / tower-http | REST API server + CORS |
| rustyline | Interactive line editor |
| enigo | Cross-platform mouse/keyboard |
| screenshots | Cross-platform screen capture |
| cpal / webrtc-vad / nnnoiseless | Voice capture + VAD + denoise |
| dirs | Platform directories |
| base64 | Image encoding for vision |

---

## Processing Loop

1. Receive a user message (CLI, --message, voice, or REST API)
2. Build a task object with skills context, system info, and known topics
3. Send to the LLM and receive a JSON response
4. If is_done is false, execute the requested skill actions
5. Feed the results back to the LLM and repeat
6. When is_done is true, return the final response
7. All messages (including intermediate steps) are saved to SQLite

Iteration and fix-iteration limits are enforced from the [execution] config.

---

## Development

```bash
cargo build              # Debug build
cargo build --release    # Optimized build
cargo check              # Fast type-check
cargo test               # Run tests
cargo clippy             # Lint
cargo fmt                # Format
```

---

## Roadmap

- [x] Memory skill with 9 retrieval methods
- [x] ShellExecutor (cross-platform, exit-code validation)
- [x] Agent loop with intermediate logging + self-correction
- [x] Session restore on startup
- [x] GuiSkill (screenshot, click, keyboard, vision AI)
- [x] WebSearchSkill (DuckDuckGo + SearXNG)
- [x] UserProfileSkill (persistent JSON profile)
- [x] Voice output (TTS) + continuous voice input (Whisper)
- [x] Web UI + REST API server mode
- [x] CLI flags (--message, --server, --voice, --help)
- [x] Supervisor lifecycle logging + backups
- [x] Context token management + retention trimming
- [ ] Comprehensive unit tests for all skills
- [ ] Full-text search (FTS5) optimization
- [ ] Persistent in-process API session (avoid per-request subprocess)
- [ ] Self-improvement engine (dynamic module generation)

---

## License

MIT License - see the LICENSE file for details.

---

Made with Rust.
