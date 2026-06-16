# IGRIS
## Intelligent General Runtime & Integrated System

[![Rust](https://img.shields.io/badge/Rust-2024%20Edition-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![CI](https://img.shields.io/github/actions/workflow/status/japo0nn/igris/release.yml?style=flat-square&logo=githubactions&label=CI)](https://github.com/japo0nn/igris/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![Status](https://img.shields.io/badge/Status-Active%20Development-brightgreen?style=flat-square)]()

> A modular, context-aware personal PC assistant written in Rust.  
> Persistent memory meets a flexible skill architecture for seamless multi-turn interactions via a JSON-based protocol.

---

## What is IGRIS?

IGRIS is an intelligent agent that lives on your machine and helps you:

- **Remembers everything** – SQLite-powered memory with 9 retrieval methods
- **Sees your screen** – Vision AI + GUI automation (screenshot, click, keyboard)
- **Searches the web** – DuckDuckGo with SearXNG fallback
- **Speaks and listens** – TTS output and continuous voice input (Whisper/Groq)
- **Executes smartly** – Multi-step actions with streaming context
- **Stays in sync** – Automatic session restoration on restart
- **Speaks JSON** – Pure JSON-based communication protocol
- **Extends easily** – Modular skill architecture (add new skills in Rust)
- **Telegram integration** – Native MTProto client for chatting via Telegram
- **Works offline-first** – OmniRoute local AI proxy included

---

## Key Features

| Feature | Status |
|---------|--------|
| 💾 Persistent SQLite memory with 9 retrieval methods | ✅ |
| 🖥️ Screen capture + Vision AI analysis | ✅ |
| 🖱️ Mouse/keyboard automation | ✅ |
| 🌐 Web search (DuckDuckGo + SearXNG) | ✅ |
| 🗣️ Text-to-speech (cross-platform) | ✅ |
| 🎤 Continuous voice input (Whisper/Groq) | ✅ |
| 📱 Telegram native client (MTProto) | ✅ |
| 🧠 Multi-step agent loop with self-correction | ✅ |
| 🔧 Extensible skill architecture | ✅ |
| 🔄 Session restore on restart | ✅ |
| 🌍 OmniRoute AI proxy (160+ providers) | ✅ |
| 📦 Pre-built binaries for all platforms | ✅ |
| 🔁 GitHub Actions CI/CD auto-release | ✅ |

---

## Quick Install

Choose your platform and run one command:

### Linux / macOS
```bash
bash <(curl -fsSL https://raw.githubusercontent.com/japo0nn/igris/main/install.sh)
```

### Windows (cmd, as Administrator)
```bat
curl -fsSLo install.bat https://raw.githubusercontent.com/japo0nn/igris/main/install.bat
install.bat
```

### Windows (PowerShell, as Administrator)
```powershell
Set-ExecutionPolicy RemoteSigned -Scope Process -Force
Invoke-WebRequest -Uri https://raw.githubusercontent.com/japo0nn/igris/main/install.ps1 -OutFile install.ps1
.\install.ps1
```

The installer will:
1. Install Rust (if missing)
2. Install Node.js 22 via fnm (if missing)
3. Install OmniRoute globally (`npm install -g omniroute@latest`)
4. Install system dependencies (Linux/macOS via apt/dnf/pacman/brew)
5. Download the latest IGRIS binary from GitHub Releases
6. Create configuration files (`~/.config/igris/config.toml` and `secrets.toml`)

> **Note:** Installers require an active internet connection. The binary is downloaded from GitHub Releases – make sure a release exists (see [Releases](https://github.com/japo0nn/igris/releases)).

---

## Build from Source

If you prefer to compile everything yourself:

```bash
# Prerequisites: Rust, Node.js 22, system deps

# Clone
cd ~
git clone https://github.com/japo0nn/igris.git
cd igris

# Install OmniRoute
git clone https://github.com/japo0nn/igris.git
# (already included in installers)
npm install -g omniroute@latest

# Build
cargo build --release

# Configure
cp secrets.toml.example secrets.toml
# Edit secrets.toml with your API keys

# Run
./target/release/igris
```

---

## Configuration

IGRIS reads `config.toml` (committed) and `secrets.toml` (git-ignored).

### config.toml

```toml
[memory]
db_path = "./igris.db"

[llm]
model = "oc/big-pickle"
base_uri = "http://localhost:20128/api"
system_prompt = "You are IGRIS ..."

[topic_llm]
model = "kr/claude-haiku-4.5"
vision_model = "cc/claude-sonnet-4-6"

[execution]
iteration_limit = 10
fix_iteration_limit = 5
```

> **Important:** Set `base_uri` to your OmniRoute endpoint. By default OmniRoute runs on `http://localhost:20128/api`.

### secrets.toml

```toml
[llm]
api_key = "sk-your-openrouter-key"

[voice]
groq_api_key = "gsk-your-groq-key"

[telegram]
api_id = 12345
api_hash = "your-api-hash"
phone_number = "+1234567890"
```

Never commit `secrets.toml` – it contains sensitive API keys.

---

## OmniRoute

[OmniRoute](https://omniroute.online/) is a local AI proxy that gives you access to **160+ LLM providers** through a single OpenAI-compatible endpoint. It runs entirely on your machine – no cloud involved.

**Why OmniRoute?**
- Bypass geographic blocks
- Auto-fallback between providers
- 15 routing strategies
- 87 MCP tools
- AES-256-GCM encrypted credentials
- Zero telemetry

IGRIS uses OmniRoute as the default LLM backend (`http://localhost:20128/api`).

### Start OmniRoute
```bash
omniroute
# or with custom port:
omniroute --port 20128
```

---

## Skills

IGRIS ships with **7 built-in skills**. Each skill exposes methods that the agent can call during execution.

### 1. Memory – Persistent Knowledge Base

All conversations persist in SQLite. Retrieve with precision:

| Method | Description | Arguments |
|--------|-------------|-----------|
| `by-topics` | Retrieve messages by topic tags | Space-separated topics |
| `get-sessions` | List all sessions | (empty) |
| `get-messages-by-time-range` | Messages within a time window | `start|end` |
| `get-messages-paginated` | Browse page-by-page | `page size` |
| `get-messages-by-session` | All messages from one session | Session UUID |
| `get-topics` | Discover all topic tags | (empty) |
| `search-messages` | Keyword search | Keyword |
| `get-message-by-id` | Fetch one message | Message UUID |
| `get-sessions-by-date` | Sessions within a date range | `start|end` |

### 2. ShellExecutor – Command Runner

Runs shell commands with exit-code validation. Windows uses PowerShell, Unix uses sh/bash.

### 3. GuiSkill – Screen Automation

| Method | Description | Arguments |
|--------|-------------|-----------|
| `screenshot` | Capture the screen | (empty) |
| `analyze_screen` | Analyze with vision AI | Question |
| `click` | Left-click at coordinates | `X Y` |
| `move_mouse` | Move cursor | `X Y` |
| `type_text` | Type text | Text |
| `scroll` | Scroll vertically | `up/down N` |
| `key_press` | Press keys/combos | `enter`, `ctrl+c`, etc. |
| `open_url` | Open URL in browser | Full URL |

Screenshots are saved to the OS temp directory.

### 4. WebSearchSkill – Internet Access

| Method | Description | Arguments |
|--------|-------------|-----------|
| `search_web` | DuckDuckGo + SearXNG fallback | Query |
| `read_page` | Extract readable text from a URL | Full URL |

### 5. UserProfileSkill – Persistent Preferences

| Method | Description | Arguments |
|--------|-------------|-----------|
| `get-profile` | Return the full profile as JSON | (empty) |
| `update-preference` | Save a key-value preference | `key|value` |
| `add-topic` | Add a topic of interest | Topic name |

Profile is stored at `~/.igris/user_profile.json` and persists across runs.

### 6. Voice – Text-to-Speech

| Method | Description | Arguments |
|--------|-------------|-----------|
| `speak` | Speak text aloud | Text |

Uses macOS `say`, Linux `espeak`/`spd-say`, or Windows PowerShell SAPI.

### 7. TelegramSkill – Native MTProto Client

| Method | Description | Arguments |
|--------|-------------|-----------|
| `status` | Check Telegram connection/auth status | (empty) |
| `login` | Start login (request SMS code) | (empty) |
| `submit_code` | Submit the login code | Code |
| `list_dialogs` | List recent chats/channels/groups | Optional limit |
| `read_chat` | Read message history from a peer | `peer|limit` |
| `send_message` | Send a text message | `peer|text` |

Uses the [ferogram](https://crates.io/crates/ferogram) crate – a native Telegram MTProto client (no Bot API needed, works with user accounts).

---

## Usage Modes

```bash
# Interactive terminal mode (default)
igris

# Single message – process and exit
igris --message "What files are on my Desktop?"
igris -m "Show me the weather"

# Continuous voice mode (microphone → Whisper → agent → TTS)
igris --voice
igris -v

# Help
igris --help
```

Interactive mode supports slash commands:
- `/help` – show available commands
- `/clear` – clear current session context
- `/history` – show session history
- `/edit` – edit the last message
- `/exit` – quit

---

## CLI Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--message <text>` | `-m` | Process a single message and exit |
| `--voice` | `-v` | Start continuous voice mode |
| `--help` | `-h` | Show help |

---

## Architecture

```
igris/
├── .github/workflows/release.yml   # CI/CD – auto-build on tags
├── src/
│   ├── main.rs                      # Entry point + CLI parsing
│   ├── registry.rs                   # Skill registration
│   ├── db.rs                        # SQLite operations
│   ├── error.rs                     # IgrisError
│   ├── supervisor.rs                # Lifecycle logging
│   ├── configs/llm.rs               # AppConfig / SecretsConfig
│   ├── core/
│   │   ├── agent.rs                 # Agent loop orchestration
│   │   ├── chat.rs                  # Interactive REPL (rustyline)
│   │   ├── llm.rs                   # LLM calls
│   │   ├── task.rs                  # Task building + topic saving
│   │   ├── markdown.rs              # Terminal markdown renderer
│   │   ├── spinner.rs               # Async progress spinner
│   │   ├── utils.rs                 # Shared helpers
│   │   └── mod.rs                   # CoreContext
│   ├── skills/
│   │   ├── memory_skill.rs          # SQLite memory (9 methods)
│   │   ├── shell_executor.rs        # Shell commands
│   │   ├── gui_skill.rs             # GUI + vision AI
│   │   ├── web_search_skill.rs      # DuckDuckGo / SearXNG
│   │   ├── user_profile_skill.rs    # Persistent profile
│   │   ├── voice_skill.rs           # TTS
│   │   ├── telegram_skill.rs        # Telegram MTProto client
│   │   └── mod.rs                   # SkillModule trait
│   ├── voice/
│   │   ├── continuous.rs            # Mic capture + Whisper
│   │   └── mod.rs
│   ├── memory/mod.rs                # Session & Message models
│   ├── models/assistant.rs          # ActionResponse, AssistantMessage
│   └── models/metadata.rs           # Skill metadata
├── config.toml                      # Main configuration
├── secrets.toml                     # API keys (git-ignored)
├── install.sh                       # Linux/macOS installer
├── install.bat                      # Windows CMD installer
├── install.ps1                      # Windows PowerShell installer
├── Cargo.toml
└── README.md
```

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime |
| `serde` / `serde_json` | Serialization |
| `toml` | Config parsing |
| `rusqlite` (bundled) | SQLite driver (no system install needed) |
| `uuid` | Unique identifiers |
| `chrono` | DateTime handling |
| `reqwest` | HTTP client (LLM, web, vision) |
| `scraper` | HTML parsing for web search |
| `rustyline` | Interactive line editor |
| `enigo` | Cross-platform mouse/keyboard |
| `screenshots` | Cross-platform screen capture |
| `cpal` / `webrtc-vad` / `nnnoiseless` | Voice capture + VAD + denoising |
| `base64` | Image encoding for vision |
| `ferogram` | Native Telegram MTProto client |
| `dirs` | Platform directories |
| `ansi-regex` | Terminal output formatting |

---

## Processing Loop

1. Receive a user message (CLI, `--message`, voice, or Telegram)
2. Build a **task object** with available skills, system info, and known topics
3. Send to the **LLM** (via OmniRoute or any OpenAI-compatible backend)
4. Receive JSON response → if `is_done: false`, execute the requested skill actions
5. Feed results back to the LLM and repeat
6. When `is_done: true`, return the final response
7. All messages (including intermediate steps) are saved to SQLite

Iteration and fix-iteration limits are enforced from `[execution]` config.

---

## CI/CD – GitHub Actions

Every time you push a tag `v*` (like `v0.1.0`) from the `main` branch:

1. **Check branch** – verifies the tag is on `main`
2. **Build** – compiles IGRIS for 4 platforms:
   - Linux x86_64
   - macOS Intel
   - macOS Apple Silicon
   - Windows x86_64
3. **Create Release** – publishes binaries with SHA256 checksums

> Releases are **only** allowed from the `main` branch. Tags pushed from other branches will fail CI.

### How to create a release
```bash
git checkout main
git pull origin main
git tag v0.1.0
git push origin v0.1.0
```

After the workflow finishes, users can install IGRIS containers.

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
- [x] CLI flags (--message, --voice, --help)
- [x] Supervisor lifecycle logging
- [x] Context token management + retention trimming
- [x] Telegram integration (native MTProto client)
- [x] OmniRoute integration (local AI proxy)
- [x] Cross-platform installers (Linux/macOS/Windows)
- [x] GitHub Actions CI/CD with auto-release
- [ ] Comprehensive unit tests for all skills
- [ ] Full-text search (FTS5) optimization
- [ ] Self-improvement engine (dynamic module generation)
- [ ] Docker support
- [ ] Desktop GUI (Tauri-based)

---

## License

MIT License – see the LICENSE file for details.

---

Made with Rust and ☕