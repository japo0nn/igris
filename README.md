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
- ⚡ **Executes smartly** — Multi-step actions with streaming context
- 🔄 **Stays in sync** — Automatic session restoration on restart
- 📝 **Speaks JSON** — Pure JSON-based communication protocol
- 🛠️ **Extends easily** — Modular skill architecture for custom extensions

---

## ✨ Key Features

### 🧠 Memory Skill — Comprehensive Knowledge Base

All conversations persist in SQLite. Query and retrieve with surgical precision:

| Method | Purpose |
|--------|----------|
| `by-topics` | Search by conversation topics |
| `get-sessions` | List all active sessions |
| `get-messages-by-time-range` | Filter by time window |
| `get-messages-paginated` | Browse messages page-by-page |
| `get-messages-by-session` | All messages from one session |
| `get-topics` | Discover all stored topics |
| `search-messages` | Full-text keyword search |
| `get-message-by-id` | Fetch specific message |
| `get-sessions-by-date` | Sessions within date range |

### ⚙️ Shell Executor — Safe Command Runner

Execute shell commands with proper error handling:

```bash
execute_command "ls -la /tmp"
```

✅ Exit code validation  
✅ Cross-platform (macOS, Linux, Windows)  
✅ Proper error messages  

### 🔄 Agent Loop — Intelligent Processing

```
User Input → Analyze Skills → Execute Actions → 
Log to DB → Return Response (JSON)
```

### 💾 Session Restore — Context Preservation

- Automatically loads last non-empty session on startup
- Maintains full conversation context between restarts
- Zero context loss guarantee

---

## 🏗️ Architecture

```
IGRIS/
├── src/
│   ├── core/
│   │   ├── agent.rs          # Agent Loop orchestration
│   │   ├── chat.rs           # Chat Loopback logic
│   │   ├── llm.rs            # LLM integration
│   │   ├── task.rs           # Task execution
│   │   └── mod.rs
│   ├── skills/
│   │   ├── memory_skill.rs    # SQLite-backed memory
│   │   ├── shell_executor.rs  # Safe shell commands
│   │   └── mod.rs
│   ├── memory/
│   │   └── mod.rs            # History management
│   ├── models/
│   │   ├── assistant.rs       # Data structures
│   │   ├── metadata.rs        # Message metadata
│   │   └── mod.rs
│   ├── configs/
│   │   ├── llm.rs            # LLM configuration
│   │   └── mod.rs
│   ├── db.rs                 # SQLite operations
│   ├── error.rs              # Error handling
│   ├── registry.rs           # Skill registration
│   └── main.rs               # Entry point
├── config.toml               # Configuration
├── secret.toml               # API keys (git-ignored)
├── Cargo.toml                # Dependencies
└── README.md                 # This file!
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

---

## 🚀 Installation

### Prerequisites
- Rust 2024 edition
- Cargo
- SQLite (bundled)

### Steps

```bash
# Clone repository
git clone git@github.com:japo0nn/igris.git
cd igris

# Build release binary
cargo build --release

# Binary ready at: ./target/release/igris
```

---

## ⚙️ Configuration

Non-sensitive settings:

```toml
[llm]
model = "claude-3-5-sonnet-20241022"
base_uri = "https://api.anthropic.com"
system_prompt = "You are IGRIS — an intelligent personal PC assistant."

[memory]
db_path = "./igris.db"
```

### Configuration `secret.toml`

**⚠️ IMPORTANT:** This file contains API keys and is **git-ignored**. Create manually:

```toml
[llm]
api_key = "your-anthropic-api-key-here"
```

**Never commit `secret.toml` to version control!**

---

## 📝 Usage

IGRIS reads from stdin and outputs JSON:

```bash
echo 'Hello IGRIS!' | ./igris
```

### Response Format

```json
{
  "message": "Your response here",
  "is_done": true,
  "actions": []
}
```

### Multi-Step Example

```json
{
  "message": "Executing shell command...",
  "is_done": false,
  "actions": [
    {
      "type": "ExecuteModule",
      "module": "ShellExecutor",
      "method": "execute_command",
      "args": "ls -la /tmp"
    }
  ]
}
```

---

## 🔄 Processing Loop

```
┌────────────────────────┐
│   User Input (JSON)    │
└───────────┬────────────┘
            │
            ▼
┌────────────────────────┐
│ Analyze Skills Context │
└───────────┬────────────┘
            │
            ▼
┌────────────────────────┐
│ Execute Actions (loop) │
└───────────┬────────────┘
            │
            ▼
┌────────────────────────┐
│ Log to SQLite Database │
└───────────┬────────────┘
            │
            ▼
┌────────────────────────┐
│ Return Final Response  │
└────────────────────────┘
```

---

## 🧪 Development

```bash
# Run tests
cargo test

# Build documentation
cargo doc --open

# Lint code
cargo clippy

# Format code
cargo fmt
```

---

## 🎯 Future Roadmap

- [ ] 🧪 Comprehensive unit tests for all skills
- [ ] 🖥️ CLI interface with `clap` (flags, configuration)
- [ ] 🌐 REST API for remote operations
- [ ] 🎨 Web dashboard for visualization
- [ ] ⚡ Full-text search (FTS5) optimization
- [ ] 📚 Developer documentation & examples
- [ ] 🔌 Plugin system for custom skills
- [ ] 🐳 Docker containerization
- [ ] 🔀 Multiple LLM provider support
- [ ] 🔐 Enhanced security features

---

## 🤝 Contributing

Contributions welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-skill`)
3. Commit changes (`git commit -am 'Add amazing skill'`)
4. Push to branch (`git push origin feature/amazing-skill`)
5. Open a Pull Request

---

## 📄 License

MIT License — see LICENSE file for details

---

## 💬 Support

Have questions? Open an issue on GitHub or check the documentation.

---

**Made with ❤️ by the IGRIS team**
