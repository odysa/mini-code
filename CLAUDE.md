# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Mini-code is a tutorial project for building a mini coding agent in Rust — a small version of tools like Claude Code or OpenCode. It has a complete reference implementation (`mini-code`) and a starter template (`mini-code-starter`) that learners fill in progressively through a 7-chapter mdBook tutorial (`mini-code-book`).

## Workspace Structure

Cargo workspace with 3 members:
- **mini-code** — Complete library: agent loop, tools (bash/read/write/edit), OpenRouter LLM provider, mock provider for tests
- **mini-code-starter** — Mirror of mini-code with empty implementations for learners to fill in
- **mini-code-xtask** — Build automation, invoked via `cargo x <command>`

## Common Commands

```bash
# Build
cargo build -p mini-code

# Run all tests
cargo test -p mini-code

# Run a single test by name
cargo test -p mini-code test_ch2_read_file

# Lint (format check + clippy)
cargo fmt --check -p mini-code
cargo clippy -p mini-code -- -D warnings

# Full solution check (fmt + clippy + tests)
cargo x solution-check

# Starter template check
cargo x check

# Build and serve the tutorial book (requires mdbook)
cargo x book
```

## Architecture

### Core Types (`mini-code/src/types.rs`)
- `Provider` trait — async interface for LLM backends (`complete` method takes messages + tool definitions)
- `Tool` trait — async interface for agent tools (`definition` returns JSON schema, `call` executes)
- `Message` enum — `User`, `Assistant`, `ToolResult` variants forming the conversation history
- `StopReason` — `Stop` (final answer) or `ToolUse` (needs tool execution)

### Agent (`mini-code/src/agent.rs`)
- `single_turn()` — One prompt → optional tool round → final response
- `SimpleAgent<P: Provider>` — Holds provider + tools, loops calling provider and executing tools until `StopReason::Stop`

### Tools (`mini-code/src/tools/`)
Each implements the `Tool` trait: `BashTool`, `ReadTool`, `WriteTool`, `EditTool`.

### Providers (`mini-code/src/providers/`)
- `OpenRouterProvider` — OpenAI-compatible HTTP provider; reads `OPENROUTER_API_KEY` from env (loaded via `dotenvy`)
- `MockProvider` (`mini-code/src/mock.rs`) — Returns pre-configured responses in sequence; used in tests

### Protocol Flow
User prompt → Provider sends prompt + tool schemas to LLM → LLM responds with text or tool calls → Agent executes tools → Tool results sent back → Loop continues until `StopReason::Stop`

## Testing

Tests live in `mini-code/src/tests/` organized by chapter (ch1.rs–ch7.rs). They use `MockProvider` to avoid real API calls and `tempfile` for filesystem tests. Both sync (`#[test]`) and async (`#[tokio::test]`) patterns are used.

## Environment

Requires a `.env` file with `OPENROUTER_API_KEY` for the live provider. The example chat app is at `mini-code/examples/chat.rs`.
