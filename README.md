# IGRIS
## Intelligent General Runtime & Integrated System

[![Rust](https://img.shields.io/badge/Rust-2024%20Edition-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![Status](https://img.shields.io/badge/Status-Active%20Development-brightgreen?style=flat-square)]()
[![Build](https://img.shields.io/badge/Build-Passing-success?style=flat-square)]()

> **A modular, context-aware personal PC assistant written in Rust.**  
> Persistent memory system meets flexible skill architecture for seamless multi-turn interactions via JSON-based messaging.

---

## 🎯 What is IGRIS?

IGRIS is your intelligent agent that:

- 🧠 **Remembers everything** — SQLite-powered memory with 9 retrieval methods
- 👁️ **Sees your screen** — Vision AI + GUI automation (screenshot, click, keyboard)
- ⚡ **Executes smartly** — Multi-step actions with streaming context
- 🔄 **Stays in sync** — Automatic session restoration on restart
- 📝 **Speaks JSON** — Pure JSON-based communication protocol
- 🛠️ **Extends easily** — Modular skill architecture for custom extensions
- 🌐 **Has a Web UI** — React frontend + REST API server mode
- 🔧 **Configurable** — max_tokens, context limits, vision models via config.toml

---

## ✨ Key Features

### 🧠 Memory Skill — Comprehensive Knowledge Base

All conversations persist in SQLite. Query and retrieve with surgical precision:

| Method | Description | Arguments |
|--------|-------------|----------|
| `by-topics` | Retrieve messages by topic tags | Space-separated topics (e.g. `birthday greeting`) |
| `get-sessions` | List all sessions with IDs and timestamps | (empty) |
| `get-messages-by-time-range` | Messages within a time window | `start\|end` timestamps |
| `get-messages-paginated` | Browse messages page-by-page | `page size` (e.g. `1 10`) |
| `get-messages-by-session` | All messages from one session | Session UUID |
| `get-topics` | Discover all stored topic tags | (empty) |
| `search-messages` | Full-text keyword search | Keyword or phrase |
| `get-message-by-id` | Fetch specific message by UUID | Message UUID |
| `get-sessions-by-date` | Sessions within date range | `start\|end` timestamps |

### ⚙️ Shell Executor — Safe Command Runner

Execute shell commands with proper error handling:

```bash
execute_command "ls -la /tmp"
```

✅ Exit code validation (not stderr guessing)  
✅ Cross-platform (macOS, Linux, Windows)  
✅ Proper error messages with exit status  

### 🖱️ GUI Skill — Screen Automation

Control your desktop with vision AI:

| Method | Description | Arguments |
|--------|-------------|----------|
| `screenshot` | Capture the current screen | (empty) |
| `analyze_screen` | Analyze screenshot with vision AI | Question/instruction |
| `click` | Left-click at pixel coordinates | `X Y` |
| `move_mouse` | Move cursor to coordinates | `X Y` |
| `type_text` | Type text at current focus | Text to type |
| `scroll` | Scroll vertically | `up/down N` |
| `key_press` | Press keys or combinations | `enter`, `cmd+c`, `ctrl+t`, etc. |
| `open_url` | Open URL in default browser | Full URL |

✅ Cross-platform (macOS, Linux, Windows)  
✅ Vision AI via configurable model (`vision_model` in config)  
✅ Screenshots saved to `/tmp/igris_screen.png`  

### 🔄 Agent Loop — Intelligent Processing

- Multi-step skill execution with result chaining
- Auto-logging of all intermediate messages to database
- Non-blocking execution (`block_in_place` for sync skills in async runtime)
- `is_done` flag tracking for completion status

### 💾 Session Restore — Context Preservation

- Automatically loads last **non-empty** session on startup
- Maintains full conversation context between restarts
- Skips empty sessions — always finds the last meaningful one

### 🌐 Web UI + REST API

Run IGRIS with a browser interface:

```bash
./start-ui.sh
```

- **React frontend** at `http://localhost:5173`
- **axum backend** at `http://localhost:3001`
- `GET /api/history` — fetch conversation history
- `POST /api/chat` — send messages via HTTP

### 📊 Context Token Management

Automatic context window management:

- `context_token_limit` — max tokens before trimming old messages
- `retention_days` — how many days of messages to keep
- Auto-trims stale messages when limit is exceeded
- Configurable per LLM provider

### 🔧 Supervisor

Lifecycle event logging:

- Logs `[STARTUP]` and `[SHUTDOWN]` events to file
- Independent from Memory Layer (no DB dependency)
- Crash-safe logging

---

## 🚀 Usage Modes

```bash
# Interactive terminal mode (default)
igris

# Single message mode — process and exit
igris --message "What files are in my Desktop?"
igris -m "Show me the weather"

# REST API server mode
igris --server
igris -s

# Full Web UI (backend + frontend)
./start-ui.sh

# Help
igris --help
```

---

## 🏗️ Architecture

```
IGRIS/
├── src/
│   ├── core/
│   │   ├── agent.rs          # Agent Loop orchestration
│   │   ├── chat.rs           # Chat Loopback (interactive mode)
│   │   ├── llm.rs            # LLM integration (ask_llm, generate_topics)
│   │   ├── task.rs           # Task/message building
│   │   └── mod.rs
│   ├── skills/
│   │   ├── memory_skill.rs    # SQLite-backed memory (9 methods)
│   │   ├── shell_executor.rs  # Safe shell commands
│   │   ├── gui_skill.rs       # GUI automation + vision AI
│   │   └── mod.rs
│   ├── memory/
│   │   └── mod.rs            # Session & message models
│   ├── models/
│   │   ├── assistant.rs       # ActionResponse, AssistantMessage, etc.
│   │   ├── metadata.rs        # ModuleMetadata
│   │   └── mod.rs
│   ├── configs/
│   │   ├── llm.rs            # AppConfig, LlmConfig, TopicLlmConfig
│   │   └── mod.rs
│   ├── api.rs                # REST API routes (axum)
│   ├── db.rs                 # SQLite operations + context management
│   ├── error.rs              # IgrisError types
│   ├── registry.rs           # Skill registration
│   ├── supervisor.rs         # Lifecycle event logging
│   └── main.rs               # Entry point + CLI flags
├── ui/                       # React frontend (Vite)
├── start-ui.sh               # Launch backend + frontend
├── config.toml               # Main configuration
├── secret.toml               # API keys (git-ignored)
├── Cargo.toml                # Dependencies
└── README.md
```

---

## ⚙️ Configuration

IGRIS comes with a pre-configured `config.toml`. You don't need to modify it to get started.

> **Note:** IGRIS uses **OmniRoute** for LLM routing capabilities. OmniRoute is an open-source local LLM proxy — see [github.com/diegosouzapw/OmniRoute](https://github.com/diegosouzapw/OmniRoute) for setup instructions.

### Setup: Create `secret.toml`

**⚠️ IMPORTANT:** This file contains API keys and is **git-ignored**. Create it manually in the project root:

```toml
[llm]
api_key = "your-omniroute-api-key-here"
```

**Never commit `secret.toml` to version control!**

### Full `config.toml` reference

```toml
[memory]
db_path = "./igris.db"

[llm]
model = "cc/claude-sonnet-4-6"
vision_model = "cc/claude-sonnet-4-6"
base_uri = "http://localhost:20128/api"
system_prompt = "..."       # IGRIS system instructions
context_token_limit = 128000 # Max tokens before trimming history
retention_days = 7           # Days to keep messages when trimming
max_tokens = 16000           # Max tokens per LLM response

[topic_llm]
model = "cc/claude-haiku-4-5-20251001"
vision_model = "cc/claude-sonnet-4-6"
system_prompt = "..."       # Topic extraction prompt
max_tokens = 1024           # Max tokens for topic extraction
```

---

## 📦 Dependencies

| Crate | Purpose |
|-------|----------|
| `tokio` | Async runtime |
| `serde_json` | JSON parsing |
| `rusqlite` | SQLite driver |
| `uuid` | Unique identifiers |
| `chrono` | DateTime handling |
| `reqwest` | HTTP client for LLM |
| `axum` | REST API server |
| `enigo` | Cross-platform mouse/keyboard |
| `xcap` | Cross-platform screen capture |

---

## 🔄 Processing Loop

1. Receive user message (via CLI, `--message`, or REST API)
2. Build Task Object with skills context, system info, all topics
3. Send to LLM → receive JSON response
4. If `is_done: false` → execute skill actions
5. Feed skill results back to LLM → repeat
6. When `is_done: true` → return final response
7. All messages (including intermediate) auto-saved to SQLite

---

## 🧪 Development

```bash
# Build
cargo build
cargo build --release

# Run tests
cargo test

# Lint
cargo clippy

# Format
cargo fmt

# Build docs
cargo doc --open
```

---

## 🎯 Roadmap

- [x] ✅ Memory Skill with 9 retrieval methods
- [x] ✅ Shell Executor (cross-platform, exit code validation)
- [x] ✅ Agent Loop with intermediate message logging
- [x] ✅ Session restore on startup
- [x] ✅ GUI Skill (screenshot, click, keyboard, vision AI)
- [x] ✅ Web UI + REST API server mode
- [x] ✅ CLI flags (`--message`, `--server`, `--help`)
- [x] ✅ Supervisor lifecycle logging
- [x] ✅ Context token limit management
- [x] ✅ Configurable max_tokens per LLM
- [x] ✅ Separate config.toml / secret.toml
- [ ] 🔜 Comprehensive unit tests for all skills
- [ ] 🔜 Full-text search (FTS5) optimization
- [ ] 🔜 Voice input (STT via Whisper/Groq)
- [ ] 🔜 Docker containerization
- [ ] 🔜 Multiple LLM provider support
- [ ] 🔜 Self-improvement engine (dynamic module generation)

---

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-skill`)
3. Commit changes (`git commit -am 'Add amazing skill'`)
4. Push to branch (`git push origin feature/amazing-skill`)
5. Open a Pull Request

---

## 📄 License

MIT License — see LICENSE file for details

---

**Made with ❤️ and Rust**
