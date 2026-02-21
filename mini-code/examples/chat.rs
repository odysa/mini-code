use mini_code::{BashTool, EditTool, OpenRouterProvider, ReadTool, SimpleAgent, WriteTool};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenRouterProvider::from_env()?;

    let agent = SimpleAgent::new(provider)
        .tool(BashTool::new())
        .tool(ReadTool::new())
        .tool(WriteTool::new())
        .tool(EditTool::new());

    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "List the files in the current directory".into());

    println!("prompt: {prompt}\n");

    let result = agent.run(&prompt).await?;
    println!("{result}");

    Ok(())
}
