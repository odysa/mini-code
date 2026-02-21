# Chapter 7: Putting It Together

> **WIP — This chapter is not yet available.**

You have built every component: a mock provider for testing, four tools, the
agent loop, and an HTTP provider. Now it is time to wire them all into a
working CLI.

The code in this chapter is about 15 lines. That is the payoff of good
abstractions -- the pieces snap together and the final program is tiny.

## Goal

Write `examples/chat.rs` in `mini-code-starter` so that:

1. It creates an `OpenRouterProvider` from environment variables.
2. It builds a `SimpleAgent` with all four tools.
3. It reads a prompt from command-line arguments.
4. It runs the agent and prints the result.

## The implementation

Open `mini-code-starter/examples/chat.rs`. You will see a skeleton with
`unimplemented!()`. Replace it with the full program.

### Step 1: Imports

You need to import the types from your crate:

```rust
use mini_code_starter::{
    BashTool, EditTool, OpenRouterProvider, ReadTool, SimpleAgent, WriteTool,
};
```

Everything is re-exported from `lib.rs`, so a single `use` statement covers it.

### Step 2: Create the provider

```rust
let provider = OpenRouterProvider::from_env()?;
```

This loads the API key from the environment (or `.env` file) and uses the
default model.

### Step 3: Build the agent

```rust
let agent = SimpleAgent::new(provider)
    .tool(BashTool::new())
    .tool(ReadTool::new())
    .tool(WriteTool::new())
    .tool(EditTool::new());
```

The builder pattern makes this clean. Each `.tool()` call registers a tool with
the agent.

### Step 4: Get the prompt

```rust
let prompt = std::env::args()
    .nth(1)
    .unwrap_or_else(|| "List the files in the current directory".into());
```

This takes the first command-line argument as the prompt, or uses a default if
none is provided.

### Step 5: Run and print

```rust
println!("prompt: {prompt}\n");
let result = agent.run(&prompt).await?;
println!("{result}");
```

### The main function

Wrap everything in an async main:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Steps 1-5 go here
    Ok(())
}
```

The `-> anyhow::Result<()>` return type lets you use `?` throughout the function.

### The complete program

Putting it all together, the entire `main` function is about 15 lines inside a
`#[tokio::main] async fn main()`. That is the beauty of the framework you
built -- the final assembly is trivial because each component has a clean
interface.

## Running the full test suite

Run the full test suite:

```bash
cargo test -p mini-code-starter
```

This runs all tests from chapters 1 through 7. If everything passes,
congratulations -- your agent framework is complete and fully tested.

### What the tests verify

The Chapter 7 tests are integration tests that combine all components:

- **Write-then-read flows**: Write a file, read it back, verify contents.
- **Edit flows**: Write a file, edit it, read back the result.
- **Multi-tool pipelines**: Use bash, write, edit, and read across multiple turns.
- **Long conversations**: Five-step tool-call sequences.

There are about 10 integration tests that exercise the full agent pipeline.

## Running the chat example

To try it with a real LLM, you need an API key. Create a `.env` file in the
workspace root:

```
OPENROUTER_API_KEY=sk-or-v1-your-key-here
```

Then run:

```bash
cargo run -p mini-code-starter --example chat -- "List the files in the current directory"
```

You should see the agent:
1. Send your prompt to the LLM.
2. The LLM decides to call the `bash` tool with something like `ls`.
3. The agent executes the command and feeds the result back.
4. The LLM formats the output into a nice response.
5. The agent prints the final response.

Try some other prompts:

```bash
cargo run -p mini-code-starter --example chat -- "Read the Cargo.toml file"
cargo run -p mini-code-starter --example chat -- "Create a file called hello.txt with a greeting"
```

## What you have built

Let's step back and look at the complete picture:

```text
examples/chat.rs
    |
    | creates
    v
SimpleAgent<OpenRouterProvider>
    |
    | holds
    +---> OpenRouterProvider (HTTP to LLM API)
    +---> ToolSet (HashMap<String, Box<dyn Tool>>)
              |
              +---> BashTool
              +---> ReadTool
              +---> WriteTool
              +---> EditTool
```

The `run()` loop drives everything:

```text
User prompt
    |
    v
Provider.chat() ---HTTP---> LLM API
    |
    | AssistantTurn
    v
Tool calls? ----yes---> Execute tools ---> feed results back ---> loop
    |
    no
    |
    v
Return text response
```

In about 300 lines of Rust across all files, you have:

- A trait-based tool system with JSON schema definitions.
- A generic agent loop that works with any provider.
- A mock provider for deterministic testing.
- An HTTP provider for real LLM APIs.
- A CLI that ties it all together.

## Where to go from here

This framework is intentionally minimal. Here are ideas for extending it:

**Streaming responses** -- Instead of waiting for the full response, stream
tokens as they arrive. This means changing `chat()` to return a `Stream` instead
of a single `AssistantTurn`.

**Conversation memory** -- Right now `run()` starts fresh each time. You could
store the message history and allow multi-turn conversations.

**More tools** -- Add a web search tool, a database query tool, or anything
else you can imagine. The `Tool` trait makes it easy to plug in new
capabilities.

**Better error handling** -- The current tools propagate errors with `?`. You
might want to catch tool errors and return them as tool results so the LLM can
try again instead of crashing.

**System prompts** -- Add a way to prepend a system message that sets the
agent's personality and instructions.

**Token limits** -- Track token usage and truncate old messages when the context
window fills up.

The foundation you built is solid. Every extension is a matter of adding to the
existing patterns, not rewriting them. The `Provider` trait, the `Tool` trait,
and the agent loop are the building blocks for anything you want to build next.

Enjoy building with your agent.
