use std::collections::VecDeque;

use serde_json::json;
use tokio::sync::mpsc;

use crate::agent::AgentEvent;
use crate::planning::PlanAgent;
use crate::streaming::MockStreamProvider;
use crate::tools::{BashTool, EditTool, ReadTool, WriteTool};
use crate::types::*;

// ---------------------------------------------------------------------------
// 1. plan() text-only response
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ch12_plan_text_response() {
    let provider = MockStreamProvider::new(VecDeque::from([AssistantTurn {
        text: Some("Here is my plan.".into()),
        tool_calls: vec![],
        stop_reason: StopReason::Stop,
    }]));

    let agent = PlanAgent::new(provider)
        .tool(ReadTool::new())
        .tool(WriteTool::new());

    let (tx, _rx) = mpsc::unbounded_channel();
    let mut messages = vec![Message::User("Plan something".into())];
    let result = agent.plan(&mut messages, tx).await.unwrap();

    assert_eq!(result, "Here is my plan.");
    assert_eq!(messages.len(), 2);
}

// ---------------------------------------------------------------------------
// 2. plan() allows read tool
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ch12_plan_with_read_tool() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("info.txt");
    std::fs::write(&path, "important data").unwrap();

    let provider = MockStreamProvider::new(VecDeque::from([
        AssistantTurn {
            text: None,
            tool_calls: vec![ToolCall {
                id: "c1".into(),
                name: "read".into(),
                arguments: json!({"path": path.to_str().unwrap()}),
            }],
            stop_reason: StopReason::ToolUse,
        },
        AssistantTurn {
            text: Some("File contains: important data".into()),
            tool_calls: vec![],
            stop_reason: StopReason::Stop,
        },
    ]));

    let agent = PlanAgent::new(provider)
        .tool(ReadTool::new())
        .tool(WriteTool::new());

    let (tx, _rx) = mpsc::unbounded_channel();
    let mut messages = vec![Message::User("Read the file".into())];
    let result = agent.plan(&mut messages, tx).await.unwrap();

    assert_eq!(result, "File contains: important data");
}

// ---------------------------------------------------------------------------
// 3. plan() blocks write tool
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ch12_plan_blocks_write_tool() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blocked.txt");

    let provider = MockStreamProvider::new(VecDeque::from([
        // LLM tries to call write during planning
        AssistantTurn {
            text: None,
            tool_calls: vec![ToolCall {
                id: "c1".into(),
                name: "write".into(),
                arguments: json!({"path": path.to_str().unwrap(), "content": "hacked"}),
            }],
            stop_reason: StopReason::ToolUse,
        },
        // LLM acknowledges the error
        AssistantTurn {
            text: Some("Cannot write in plan mode.".into()),
            tool_calls: vec![],
            stop_reason: StopReason::Stop,
        },
    ]));

    let agent = PlanAgent::new(provider)
        .tool(ReadTool::new())
        .tool(WriteTool::new());

    let (tx, _rx) = mpsc::unbounded_channel();
    let mut messages = vec![Message::User("Write a file".into())];
    let result = agent.plan(&mut messages, tx).await.unwrap();

    assert_eq!(result, "Cannot write in plan mode.");
    // File must NOT have been created
    assert!(!path.exists());

    // Verify the error tool result was sent back
    let tool_result = messages
        .iter()
        .find(|m| matches!(m, Message::ToolResult { .. }));
    assert!(tool_result.is_some());
    if let Some(Message::ToolResult { content, .. }) = tool_result {
        assert!(content.contains("not available in planning mode"));
    }
}

// ---------------------------------------------------------------------------
// 4. plan() blocks edit tool
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ch12_plan_blocks_edit_tool() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("target.txt");
    std::fs::write(&path, "original").unwrap();

    let provider = MockStreamProvider::new(VecDeque::from([
        AssistantTurn {
            text: None,
            tool_calls: vec![ToolCall {
                id: "c1".into(),
                name: "edit".into(),
                arguments: json!({
                    "path": path.to_str().unwrap(),
                    "old": "original",
                    "new": "modified"
                }),
            }],
            stop_reason: StopReason::ToolUse,
        },
        AssistantTurn {
            text: Some("Edit blocked.".into()),
            tool_calls: vec![],
            stop_reason: StopReason::Stop,
        },
    ]));

    let agent = PlanAgent::new(provider)
        .tool(ReadTool::new())
        .tool(EditTool::new());

    let (tx, _rx) = mpsc::unbounded_channel();
    let mut messages = vec![Message::User("Edit the file".into())];
    let result = agent.plan(&mut messages, tx).await.unwrap();

    assert_eq!(result, "Edit blocked.");
    // File must be unchanged
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
}

// ---------------------------------------------------------------------------
// 5. execute() allows write tool
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ch12_execute_allows_write_tool() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("output.txt");

    let provider = MockStreamProvider::new(VecDeque::from([
        AssistantTurn {
            text: None,
            tool_calls: vec![ToolCall {
                id: "c1".into(),
                name: "write".into(),
                arguments: json!({"path": path.to_str().unwrap(), "content": "written!"}),
            }],
            stop_reason: StopReason::ToolUse,
        },
        AssistantTurn {
            text: Some("File written.".into()),
            tool_calls: vec![],
            stop_reason: StopReason::Stop,
        },
    ]));

    let agent = PlanAgent::new(provider)
        .tool(ReadTool::new())
        .tool(WriteTool::new());

    let (tx, _rx) = mpsc::unbounded_channel();
    let mut messages = vec![Message::User("Approved. Execute.".into())];
    let result = agent.execute(&mut messages, tx).await.unwrap();

    assert_eq!(result, "File written.");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "written!");
}

// ---------------------------------------------------------------------------
// 6. Full plan-then-execute flow
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ch12_full_plan_then_execute() {
    let dir = tempfile::tempdir().unwrap();
    let read_path = dir.path().join("source.txt");
    let write_path = dir.path().join("dest.txt");
    std::fs::write(&read_path, "source data").unwrap();

    // Plan phase: LLM reads the source file, then responds with plan text
    // Execute phase: LLM writes the dest file, then responds with done text
    let provider = MockStreamProvider::new(VecDeque::from([
        // Plan turn 1: read file
        AssistantTurn {
            text: None,
            tool_calls: vec![ToolCall {
                id: "c1".into(),
                name: "read".into(),
                arguments: json!({"path": read_path.to_str().unwrap()}),
            }],
            stop_reason: StopReason::ToolUse,
        },
        // Plan turn 2: return plan
        AssistantTurn {
            text: Some("Plan: copy source data to dest.txt".into()),
            tool_calls: vec![],
            stop_reason: StopReason::Stop,
        },
        // Execute turn 1: write file
        AssistantTurn {
            text: None,
            tool_calls: vec![ToolCall {
                id: "c2".into(),
                name: "write".into(),
                arguments: json!({"path": write_path.to_str().unwrap(), "content": "source data"}),
            }],
            stop_reason: StopReason::ToolUse,
        },
        // Execute turn 2: done
        AssistantTurn {
            text: Some("Done. Copied to dest.txt".into()),
            tool_calls: vec![],
            stop_reason: StopReason::Stop,
        },
    ]));

    let agent = PlanAgent::new(provider)
        .tool(ReadTool::new())
        .tool(WriteTool::new());

    let (tx, _rx) = mpsc::unbounded_channel();
    let mut messages = vec![Message::User("Copy source.txt to dest.txt".into())];

    // Phase 1: Plan
    let plan = agent.plan(&mut messages, tx).await.unwrap();
    assert_eq!(plan, "Plan: copy source data to dest.txt");
    assert!(!write_path.exists()); // not written yet

    // Phase 2: Approve and execute
    messages.push(Message::User("Approved. Execute.".into()));
    let (tx2, _rx2) = mpsc::unbounded_channel();
    let result = agent.execute(&mut messages, tx2).await.unwrap();
    assert_eq!(result, "Done. Copied to dest.txt");
    assert_eq!(std::fs::read_to_string(&write_path).unwrap(), "source data");
}

// ---------------------------------------------------------------------------
// 7. Message continuity between phases
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ch12_message_continuity() {
    let provider = MockStreamProvider::new(VecDeque::from([
        // Plan phase
        AssistantTurn {
            text: Some("My plan".into()),
            tool_calls: vec![],
            stop_reason: StopReason::Stop,
        },
        // Execute phase
        AssistantTurn {
            text: Some("Executed".into()),
            tool_calls: vec![],
            stop_reason: StopReason::Stop,
        },
    ]));

    let agent = PlanAgent::new(provider).tool(ReadTool::new());

    let (tx, _rx) = mpsc::unbounded_channel();
    let mut messages = vec![Message::User("Task".into())];

    let _ = agent.plan(&mut messages, tx).await.unwrap();
    // After plan: [User, Assistant]
    assert_eq!(messages.len(), 2);

    messages.push(Message::User("Approved".into()));
    // Before execute: [User, Assistant, User]
    assert_eq!(messages.len(), 3);

    let (tx2, _rx2) = mpsc::unbounded_channel();
    let _ = agent.execute(&mut messages, tx2).await.unwrap();
    // After execute: [User, Assistant, User, Assistant]
    assert_eq!(messages.len(), 4);
}

// ---------------------------------------------------------------------------
// 8. read_only override
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ch12_read_only_override() {
    // Override read_only to only include "read" (no "bash")
    let provider = MockStreamProvider::new(VecDeque::from([
        // LLM tries bash during planning
        AssistantTurn {
            text: None,
            tool_calls: vec![ToolCall {
                id: "c1".into(),
                name: "bash".into(),
                arguments: json!({"command": "rm -rf /"}),
            }],
            stop_reason: StopReason::ToolUse,
        },
        AssistantTurn {
            text: Some("Bash blocked.".into()),
            tool_calls: vec![],
            stop_reason: StopReason::Stop,
        },
    ]));

    let agent = PlanAgent::new(provider)
        .tool(ReadTool::new())
        .tool(BashTool::new())
        .read_only(&["read"]); // bash excluded

    let (tx, _rx) = mpsc::unbounded_channel();
    let mut messages = vec![Message::User("Plan".into())];
    let result = agent.plan(&mut messages, tx).await.unwrap();

    assert_eq!(result, "Bash blocked.");
    // Verify error was sent back
    let tool_result = messages
        .iter()
        .find(|m| matches!(m, Message::ToolResult { .. }));
    assert!(tool_result.is_some());
    if let Some(Message::ToolResult { content, .. }) = tool_result {
        assert!(content.contains("not available in planning mode"));
    }
}

// ---------------------------------------------------------------------------
// 9. Streaming events during plan
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ch12_streaming_events_during_plan() {
    let provider = MockStreamProvider::new(VecDeque::from([AssistantTurn {
        text: Some("Plan text".into()),
        tool_calls: vec![],
        stop_reason: StopReason::Stop,
    }]));

    let agent = PlanAgent::new(provider).tool(ReadTool::new());

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut messages = vec![Message::User("Plan".into())];
    let _ = agent.plan(&mut messages, tx).await.unwrap();

    let mut events = Vec::new();
    while let Ok(e) = rx.try_recv() {
        events.push(e);
    }

    // Should have TextDelta events (MockStreamProvider sends one per char)
    assert!(events.iter().any(|e| matches!(e, AgentEvent::TextDelta(_))));
    // Should end with Done
    assert!(events.iter().any(|e| matches!(e, AgentEvent::Done(_))));
}

// ---------------------------------------------------------------------------
// 10. Provider error propagated
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ch12_plan_provider_error() {
    // Empty mock → error on first call
    let provider = MockStreamProvider::new(VecDeque::new());
    let agent = PlanAgent::new(provider).tool(ReadTool::new());

    let (tx, _rx) = mpsc::unbounded_channel();
    let mut messages = vec![Message::User("Plan".into())];
    let result = agent.plan(&mut messages, tx).await;

    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// 11. Builder pattern compile test
// ---------------------------------------------------------------------------

#[test]
fn test_ch12_builder_pattern() {
    let provider = MockStreamProvider::new(VecDeque::new());
    let _agent = PlanAgent::new(provider)
        .tool(ReadTool::new())
        .tool(WriteTool::new())
        .tool(EditTool::new())
        .tool(BashTool::new())
        .read_only(&["read", "bash"]);
    // If this compiles and runs, the builder pattern works.
}
