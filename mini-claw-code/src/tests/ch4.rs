use crate::tools::{BashTool, EditTool, WriteTool};
use crate::types::*;
use serde_json::json;

// ---------------------------------------------------------------------------
// BashTool
// ---------------------------------------------------------------------------

#[test]
fn test_ch4_bash_definition() {
    let tool = BashTool::new();
    let def = tool.definition();
    assert_eq!(def.name, "bash");
    assert!(!def.description.is_empty());
    let required = def.parameters["required"].as_array().unwrap();
    assert!(required.iter().any(|v| v == "command"));
}

#[tokio::test]
async fn test_ch4_bash_runs_command() {
    let tool = BashTool::new();
    let result = tool.call(json!({"command": "echo hello"})).await.unwrap();
    assert!(result.contains("hello"));
}

#[tokio::test]
async fn test_ch4_bash_captures_stderr() {
    let tool = BashTool::new();
    let result = tool.call(json!({"command": "echo err >&2"})).await.unwrap();
    assert!(result.contains("err"));
}

#[tokio::test]
async fn test_ch4_bash_missing_arg() {
    let tool = BashTool::new();
    let result = tool.call(json!({})).await;
    assert!(result.is_err());
}

// --- New BashTool tests ---

#[test]
fn test_ch4_bash_default() {
    let tool = BashTool::default();
    assert_eq!(tool.definition().name, "bash");
}

#[tokio::test]
async fn test_ch4_bash_stdout_and_stderr() {
    let tool = BashTool::new();
    let result = tool
        .call(json!({"command": "echo out && echo err >&2"}))
        .await
        .unwrap();
    assert!(result.contains("out"));
    assert!(result.contains("stderr:"));
    assert!(result.contains("err"));
}

#[tokio::test]
async fn test_ch4_bash_no_output() {
    let tool = BashTool::new();
    let result = tool.call(json!({"command": "true"})).await.unwrap();
    assert_eq!(result, "(no output)");
}

#[tokio::test]
async fn test_ch4_bash_exit_code_nonzero() {
    // bash tool still returns output even for non-zero exit code
    let tool = BashTool::new();
    let result = tool
        .call(json!({"command": "echo fail && exit 1"}))
        .await
        .unwrap();
    assert!(result.contains("fail"));
}

#[tokio::test]
async fn test_ch4_bash_multiline_output() {
    let tool = BashTool::new();
    let result = tool
        .call(json!({"command": "echo line1 && echo line2 && echo line3"}))
        .await
        .unwrap();
    assert!(result.contains("line1"));
    assert!(result.contains("line2"));
    assert!(result.contains("line3"));
}

#[tokio::test]
async fn test_ch4_bash_wrong_arg_type() {
    let tool = BashTool::new();
    let result = tool.call(json!({"command": 123})).await;
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// WriteTool
// ---------------------------------------------------------------------------

#[test]
fn test_ch4_write_definition() {
    let tool = WriteTool::new();
    let def = tool.definition();
    assert_eq!(def.name, "write");
    assert!(!def.description.is_empty());
}

#[tokio::test]
async fn test_ch4_write_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.txt");

    let tool = WriteTool::new();
    tool.call(json!({"path": path.to_str().unwrap(), "content": "hello"}))
        .await
        .unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
}

#[tokio::test]
async fn test_ch4_write_creates_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a/b/c/out.txt");

    let tool = WriteTool::new();
    tool.call(json!({"path": path.to_str().unwrap(), "content": "deep"}))
        .await
        .unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "deep");
}

#[tokio::test]
async fn test_ch4_write_missing_arg() {
    let tool = WriteTool::new();
    let result = tool.call(json!({"path": "/tmp/x.txt"})).await;
    assert!(result.is_err());
}

// --- New WriteTool tests ---

#[test]
fn test_ch4_write_default() {
    let tool = WriteTool::default();
    assert_eq!(tool.definition().name, "write");
}

#[test]
fn test_ch4_write_definition_required_params() {
    let tool = WriteTool::new();
    let def = tool.definition();
    let required = def.parameters["required"].as_array().unwrap();
    assert!(required.iter().any(|v| v == "path"));
    assert!(required.iter().any(|v| v == "content"));
}

#[tokio::test]
async fn test_ch4_write_overwrites_existing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("overwrite.txt");
    std::fs::write(&path, "old content").unwrap();

    let tool = WriteTool::new();
    tool.call(json!({"path": path.to_str().unwrap(), "content": "new content"}))
        .await
        .unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new content");
}

#[tokio::test]
async fn test_ch4_write_empty_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.txt");

    let tool = WriteTool::new();
    tool.call(json!({"path": path.to_str().unwrap(), "content": ""}))
        .await
        .unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
}

#[tokio::test]
async fn test_ch4_write_returns_confirmation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("confirm.txt");

    let tool = WriteTool::new();
    let result = tool
        .call(json!({"path": path.to_str().unwrap(), "content": "data"}))
        .await
        .unwrap();

    assert!(result.contains("wrote"));
    assert!(result.contains(path.to_str().unwrap()));
}

#[tokio::test]
async fn test_ch4_write_missing_path() {
    let tool = WriteTool::new();
    let result = tool.call(json!({"content": "data"})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_ch4_write_multiline_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("multi.txt");

    let tool = WriteTool::new();
    tool.call(json!({"path": path.to_str().unwrap(), "content": "line1\nline2\nline3"}))
        .await
        .unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert_eq!(content, "line1\nline2\nline3");
}

// ---------------------------------------------------------------------------
// EditTool
// ---------------------------------------------------------------------------

#[test]
fn test_ch4_edit_definition() {
    let tool = EditTool::new();
    let def = tool.definition();
    assert_eq!(def.name, "edit");
    assert!(!def.description.is_empty());
}

#[tokio::test]
async fn test_ch4_edit_replaces_string() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("edit.txt");
    std::fs::write(&path, "hello world").unwrap();

    let tool = EditTool::new();
    tool.call(json!({
        "path": path.to_str().unwrap(),
        "old_string": "hello",
        "new_string": "goodbye"
    }))
    .await
    .unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "goodbye world");
}

#[tokio::test]
async fn test_ch4_edit_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("edit.txt");
    std::fs::write(&path, "hello world").unwrap();

    let tool = EditTool::new();
    let result = tool
        .call(json!({
            "path": path.to_str().unwrap(),
            "old_string": "missing",
            "new_string": "replacement"
        }))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_ch4_edit_not_unique() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("edit.txt");
    std::fs::write(&path, "aaa").unwrap();

    let tool = EditTool::new();
    let result = tool
        .call(json!({
            "path": path.to_str().unwrap(),
            "old_string": "a",
            "new_string": "b"
        }))
        .await;

    assert!(result.is_err());
}

// --- New EditTool tests ---

#[test]
fn test_ch4_edit_default() {
    let tool = EditTool::default();
    assert_eq!(tool.definition().name, "edit");
}

#[test]
fn test_ch4_edit_definition_required_params() {
    let tool = EditTool::new();
    let def = tool.definition();
    let required = def.parameters["required"].as_array().unwrap();
    assert!(required.iter().any(|v| v == "path"));
    assert!(required.iter().any(|v| v == "old_string"));
    assert!(required.iter().any(|v| v == "new_string"));
}

#[tokio::test]
async fn test_ch4_edit_returns_confirmation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("confirm.txt");
    std::fs::write(&path, "foo bar").unwrap();

    let tool = EditTool::new();
    let result = tool
        .call(json!({
            "path": path.to_str().unwrap(),
            "old_string": "foo",
            "new_string": "baz"
        }))
        .await
        .unwrap();

    assert!(result.contains("edited"));
    assert!(result.contains(path.to_str().unwrap()));
}

#[tokio::test]
async fn test_ch4_edit_missing_file() {
    let tool = EditTool::new();
    let result = tool
        .call(json!({
            "path": "/tmp/__mini_claw_code_no_such_file_ch4__.txt",
            "old_string": "old",
            "new_string": "new"
        }))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_ch4_edit_replace_with_empty() {
    // Effectively delete a substring
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("del.txt");
    std::fs::write(&path, "remove this part").unwrap();

    let tool = EditTool::new();
    tool.call(json!({
        "path": path.to_str().unwrap(),
        "old_string": " this part",
        "new_string": ""
    }))
    .await
    .unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "remove");
}

#[tokio::test]
async fn test_ch4_edit_replace_with_longer() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("grow.txt");
    std::fs::write(&path, "short").unwrap();

    let tool = EditTool::new();
    tool.call(json!({
        "path": path.to_str().unwrap(),
        "old_string": "short",
        "new_string": "a much longer replacement string"
    }))
    .await
    .unwrap();

    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "a much longer replacement string"
    );
}

#[tokio::test]
async fn test_ch4_edit_multiline_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("multi.txt");
    std::fs::write(&path, "line1\nline2\nline3").unwrap();

    let tool = EditTool::new();
    tool.call(json!({
        "path": path.to_str().unwrap(),
        "old_string": "line2",
        "new_string": "replaced2\nextra_line"
    }))
    .await
    .unwrap();

    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "line1\nreplaced2\nextra_line\nline3"
    );
}

#[tokio::test]
async fn test_ch4_edit_missing_old_string_arg() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("edit.txt");
    std::fs::write(&path, "content").unwrap();

    let tool = EditTool::new();
    let result = tool
        .call(json!({
            "path": path.to_str().unwrap(),
            "new_string": "replacement"
        }))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_ch4_edit_missing_new_string_arg() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("edit.txt");
    std::fs::write(&path, "content").unwrap();

    let tool = EditTool::new();
    let result = tool
        .call(json!({
            "path": path.to_str().unwrap(),
            "old_string": "content"
        }))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_ch4_edit_preserves_rest_of_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("preserve.txt");
    std::fs::write(&path, "header\ntarget_line\nfooter").unwrap();

    let tool = EditTool::new();
    tool.call(json!({
        "path": path.to_str().unwrap(),
        "old_string": "target_line",
        "new_string": "replaced_line"
    }))
    .await
    .unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.starts_with("header\n"));
    assert!(content.contains("replaced_line"));
    assert!(content.ends_with("\nfooter"));
}
