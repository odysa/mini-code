use std::io::{self, BufRead, Write};
use std::sync::Arc;
use std::time::Duration;

use mini_claw_code::{
    AgentEvent, AskTool, BashTool, ChannelInputHandler, EditTool, Message, OpenRouterProvider,
    ReadTool, StreamingAgent, UserInputRequest, WriteTool,
};
use tokio::sync::mpsc;

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

// ANSI helpers
const BOLD_CYAN: &str = "\x1b[1;36m";
const BOLD_MAGENTA: &str = "\x1b[1;35m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";
const CLEAR_LINE: &str = "\x1b[2K\r";

/// Present options as an arrow-key-navigable list using crossterm raw mode.
///
/// Returns the selected option string, or switches to free-text if the user
/// types any letter.
fn select_option(question: &str, options: &[String]) -> io::Result<String> {
    use crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyEvent},
        terminal,
    };

    terminal::enable_raw_mode()?;

    let mut selected: usize = 0;
    let mut stdout = io::stdout();

    // Draw initial list
    write!(stdout, "\r\n  {BOLD_CYAN}{question}{RESET}\r\n")?;
    for (i, opt) in options.iter().enumerate() {
        if i == selected {
            write!(stdout, "  {BOLD_CYAN}> {opt}{RESET}\r\n")?;
        } else {
            write!(stdout, "    {opt}\r\n")?;
        }
    }
    stdout.flush()?;

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Up => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if selected + 1 < options.len() {
                            selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        terminal::disable_raw_mode()?;
                        // Move past the list
                        write!(
                            stdout,
                            "\r{CLEAR_LINE}  {DIM}> {}{RESET}\r\n",
                            options[selected]
                        )?;
                        stdout.flush()?;
                        return Ok(options[selected].clone());
                    }
                    KeyCode::Char(_) => {
                        // Switch to free-text mode
                        terminal::disable_raw_mode()?;
                        write!(stdout, "\r{CLEAR_LINE}  > ")?;
                        stdout.flush()?;
                        let mut line = String::new();
                        io::stdin().lock().read_line(&mut line)?;
                        return Ok(line.trim().to_string());
                    }
                    KeyCode::Esc => {
                        terminal::disable_raw_mode()?;
                        return Ok(options[selected].clone());
                    }
                    _ => {}
                }

                // Redraw list — move cursor up to start of list
                write!(stdout, "{}", cursor::MoveUp(options.len() as u16))?;
                for (i, opt) in options.iter().enumerate() {
                    if i == selected {
                        write!(stdout, "\r{CLEAR_LINE}  {BOLD_CYAN}> {opt}{RESET}\r\n")?;
                    } else {
                        write!(stdout, "\r{CLEAR_LINE}    {opt}\r\n")?;
                    }
                }
                stdout.flush()?;
            }
        }
    }
}

/// Handle a UserInputRequest: either show arrow-key selection or a simple prompt.
fn handle_input_request(req: UserInputRequest) {
    let answer = if req.options.is_empty() {
        // Simple text prompt
        print!("\n  {BOLD_CYAN}{}{RESET}\n  > ", req.question);
        let _ = io::stdout().flush();
        let mut line = String::new();
        let _ = io::stdin().lock().read_line(&mut line);
        line.trim().to_string()
    } else {
        // Arrow-key selection
        match select_option(&req.question, &req.options) {
            Ok(s) => s,
            Err(_) => req.options.first().cloned().unwrap_or_default(),
        }
    };
    let _ = req.response_tx.send(answer);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenRouterProvider::from_env_with_model("anthropic/claude-opus-4-5")?;

    // Channel for AskTool → TUI communication
    let (input_tx, mut input_rx) = mpsc::unbounded_channel::<UserInputRequest>();
    let handler = Arc::new(ChannelInputHandler::new(input_tx));

    let agent = Arc::new(
        StreamingAgent::new(provider)
            .tool(BashTool::new())
            .tool(ReadTool::new())
            .tool(WriteTool::new())
            .tool(EditTool::new())
            .tool(AskTool::new(handler)),
    );

    let stdin = io::stdin();
    let mut history: Vec<Message> = Vec::new();
    println!();

    loop {
        print!("{BOLD_CYAN}>{RESET} ");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            println!();
            break;
        }
        let prompt = line.trim().to_string();
        if prompt.is_empty() {
            continue;
        }
        println!();

        // Append user message and spawn streaming agent task
        history.push(Message::User(prompt));
        let (tx, mut rx) = mpsc::unbounded_channel();
        let agent = agent.clone();
        let mut msgs = std::mem::take(&mut history);
        let handle = tokio::spawn(async move {
            let _ = agent.chat(&mut msgs, tx).await;
            msgs
        });

        // UI event loop
        let mut tick = tokio::time::interval(Duration::from_millis(80));
        let mut frame = 0usize;
        let mut tool_count = 0usize;
        let mut streaming_text = false;
        let mut text_buf = String::new();
        const COLLAPSE_AFTER: usize = 3;

        // Initial spinner
        print!(
            "{BOLD_MAGENTA}⏺{RESET} {YELLOW}{} Thinking...{RESET}",
            SPINNER[0]
        );
        let _ = io::stdout().flush();

        loop {
            tokio::select! {
                event = rx.recv() => {
                    match event {
                        Some(AgentEvent::TextDelta(text)) => {
                            streaming_text = true;
                            text_buf.push_str(&text);
                        }
                        Some(AgentEvent::ToolCall { summary, .. }) => {
                            tool_count += 1;
                            streaming_text = false;
                            text_buf.clear();

                            if tool_count <= COLLAPSE_AFTER {
                                print!("{CLEAR_LINE}  {DIM}⎿  {summary}{RESET}\n");
                            } else if tool_count == COLLAPSE_AFTER + 1 {
                                print!("{CLEAR_LINE}  {DIM}⎿  ... and 1 more{RESET}\n");
                            } else {
                                let extra = tool_count - COLLAPSE_AFTER;
                                print!("{CLEAR_LINE}\x1b[A{CLEAR_LINE}  {DIM}⎿  ... and {extra} more{RESET}\n");
                            }

                            let ch = SPINNER[frame % SPINNER.len()];
                            print!("{BOLD_MAGENTA}⏺{RESET} {YELLOW}{ch} Thinking...{RESET}");
                            let _ = io::stdout().flush();
                        }
                        Some(AgentEvent::Done(_)) => {
                            print!("{CLEAR_LINE}");
                            let _ = io::stdout().flush();
                            if !text_buf.is_empty() {
                                let rendered = termimad::text(&text_buf);
                                print!("{rendered}\n");
                            } else {
                                println!();
                            }
                            break;
                        }
                        Some(AgentEvent::Error(e)) => {
                            print!("{CLEAR_LINE}");
                            let _ = io::stdout().flush();
                            if tool_count > 0 { println!(); }
                            println!("{BOLD_MAGENTA}⏺{RESET} {RED}error: {e}{RESET}\n");
                            break;
                        }
                        None => {
                            print!("{CLEAR_LINE}");
                            let _ = io::stdout().flush();
                            break;
                        }
                    }
                }
                // Handle user input requests from AskTool
                Some(req) = input_rx.recv() => {
                    // Clear spinner line before showing the question
                    print!("{CLEAR_LINE}");
                    let _ = io::stdout().flush();
                    streaming_text = false;

                    // Handle input on a blocking thread (reads from stdin)
                    tokio::task::spawn_blocking(move || handle_input_request(req))
                        .await
                        .ok();

                    // Restore spinner
                    let ch = SPINNER[frame % SPINNER.len()];
                    print!("{BOLD_MAGENTA}⏺{RESET} {YELLOW}{ch} Thinking...{RESET}");
                    let _ = io::stdout().flush();
                }
                _ = tick.tick() => {
                    if !streaming_text {
                        frame += 1;
                        let ch = SPINNER[frame % SPINNER.len()];
                        print!("\r{BOLD_MAGENTA}⏺{RESET} {YELLOW}{ch} Thinking...{RESET}");
                        let _ = io::stdout().flush();
                    }
                }
            }
        }

        // Recover conversation history from the agent task
        if let Ok(msgs) = handle.await {
            history = msgs;
        }
    }

    Ok(())
}
