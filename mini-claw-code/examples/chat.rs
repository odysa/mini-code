use std::io::{self, BufRead, Write};
use std::sync::Arc;

use mini_claw_code::{
    AskTool, BashTool, CliInputHandler, EditTool, Message, OpenRouterProvider, ReadTool,
    SimpleAgent, WriteTool,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenRouterProvider::from_env()?;
    let agent = SimpleAgent::new(provider)
        .tool(BashTool::new())
        .tool(ReadTool::new())
        .tool(WriteTool::new())
        .tool(EditTool::new())
        .tool(AskTool::new(Arc::new(CliInputHandler)));

    let cwd = std::env::current_dir()?.display().to_string();
    let stdin = io::stdin();
    let mut history: Vec<Message> = vec![Message::System(format!(
        "You are a coding agent. Help the user with software engineering tasks \
         using all available tools. Be concise and precise.\n\n\
         Working directory: {cwd}"
    ))];

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            println!();
            break;
        }

        let prompt = line.trim();
        if prompt.is_empty() {
            continue;
        }

        history.push(Message::User(prompt.to_string()));
        print!("    thinking...");
        io::stdout().flush()?;
        match agent.chat(&mut history).await {
            Ok(text) => {
                print!("\x1b[2K\r");
                println!("{}\n", text.trim());
            }
            Err(e) => {
                print!("\x1b[2K\r");
                println!("error: {e}\n");
            }
        }
    }

    Ok(())
}
