// Chapter 7: The CLI
//
// Build an interactive CLI that reads prompts in a loop and runs the agent.
//
// Steps:
// 1. Import types: BashTool, EditTool, Message, OpenRouterProvider, ReadTool, SimpleAgent, WriteTool
// 2. Create an OpenRouterProvider using from_env()
// 3. Build a SimpleAgent with all four tools (Bash, Read, Write, Edit)
// 4. Create a Vec<Message> to hold the conversation history
// 5. Loop: print "> ", read a line from stdin, push Message::User, call agent.chat(), print result
// 6. Break on EOF (Ctrl+D)

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    unimplemented!(
        "Create provider, build agent with tools, loop reading stdin, push to history, call chat(), print result"
    )
}
