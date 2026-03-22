# igris

> A self-evolving AI assistant built in Rust — designed to autonomously develop, connect, and extend its own capabilities over time.

---

## What is this?

Most AI assistants are static. You install them, they do what they were built to do, and that's it.

**igris** is an experiment in a different direction: an assistant that can grow itself. The long-term goal is a system that identifies what it's missing, writes or connects additional modules, and integrates them — without manual intervention.

Right now it's early. But the foundation is there.

---

## Current state

- Terminal-based chat interface (no voice yet)
- Conversational memory within a session via context window
- Persistent message storage with SQLite
- Powered by an LLM backend via API

This is a working prototype. The core loop — **write, respond, remember** — is functional.

---

## Roadmap

- [ ] Voice input / output
- [ ] Plugin / module system (dynamic loading)
- [ ] Self-modification: assistant proposes and integrates new capabilities
- [ ] Persistent cross-session memory with retrieval
- [ ] Tool use (web search, file system, code execution)
- [ ] Local LLM support (offline mode)

---

## Getting started

```bash
git clone https://github.com/japo0nn/igris
cd igris
cargo run
```

> Requires Rust stable. API key configuration — see `.env.example` (coming soon).

---

## Why Rust?

Performance, safety, and because building something this close to the metal in a memory-safe language felt right for a system that's supposed to manage itself.

---

## Status

Early development. Things will break, change, and hopefully get smarter.
