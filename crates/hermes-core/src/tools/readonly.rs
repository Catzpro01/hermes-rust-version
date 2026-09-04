use super::{Tool, ToolCall, ToolError, ToolResponse};
use async_trait::async_trait;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio_util::sync::CancellationToken;

const MAX_FILE_BYTES: usize = 100_000;
const MAX_ENTRIES: usize = 500;

fn arg_path(arguments: &str) -> Result<String, ToolError> {
    if let Ok(value) = serde_json::from_str::<Value>(arguments) {
        return value
            .get("path")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| ToolError::Failed("missing string argument: path".into()));
    }
    arguments
        .strip_prefix("path:")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ToolError::Failed("expected path argument".into()))
}
async fn safe_path(root: &Path, requested: &str) -> Result<PathBuf, ToolError> {
    let root = fs::canonicalize(root)
        .await
        .map_err(|e| ToolError::Failed(format!("invalid tool root: {e}")))?;
    if Path::new(requested)
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(ToolError::Denied("path traversal is not allowed".into()));
    }
    let candidate = if Path::new(requested).is_absolute() {
        PathBuf::from(requested)
    } else {
        root.join(requested)
    };
    let resolved = fs::canonicalize(&candidate)
        .await
        .map_err(|e| ToolError::Failed(format!("path unavailable: {e}")))?;
    if !resolved.starts_with(&root) {
        return Err(ToolError::Denied("path is outside the tool root".into()));
    }
    Ok(resolved)
}

pub struct ReadFileTool {
    root: PathBuf,
}
impl ReadFileTool {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}
#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }
    fn description(&self) -> &str {
        "Read a UTF-8 file under the tool root, limited to 100 KB."
    }
    async fn execute(
        &self,
        call: &ToolCall,
        cancel: CancellationToken,
    ) -> Result<ToolResponse, ToolError> {
        let path = safe_path(&self.root, &arg_path(&call.arguments)?).await?;
        let bytes = tokio::select! { _ = cancel.cancelled() => return Err(ToolError::Cancelled), result = fs::read(&path) => result.map_err(|e| ToolError::Failed(e.to_string()))? };
        let truncated = bytes.len() > MAX_FILE_BYTES;
        let end = bytes.len().min(MAX_FILE_BYTES);
        let content = String::from_utf8_lossy(&bytes[..end]).to_string()
            + if truncated {
                "\n[truncated at 100 KB]"
            } else {
                ""
            };
        Ok(ToolResponse {
            id: call.id.clone(),
            name: call.name.clone(),
            content,
            success: true,
        })
    }
}

pub struct ListDirTool {
    root: PathBuf,
}
impl ListDirTool {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}
#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }
    fn description(&self) -> &str {
        "List entries under the tool root, limited to 500 entries."
    }
    async fn execute(
        &self,
        call: &ToolCall,
        cancel: CancellationToken,
    ) -> Result<ToolResponse, ToolError> {
        let requested = arg_path(&call.arguments)?;
        let path = safe_path(&self.root, &requested).await?;
        let mut dir = fs::read_dir(path)
            .await
            .map_err(|e| ToolError::Failed(e.to_string()))?;
        let mut lines = Vec::new();
        loop {
            let entry = tokio::select! { _ = cancel.cancelled() => return Err(ToolError::Cancelled), result = dir.next_entry() => result.map_err(|e| ToolError::Failed(e.to_string()))? };
            let Some(entry) = entry else { break };
            if lines.len() == MAX_ENTRIES {
                lines.push("[truncated at 500 entries]".into());
                break;
            }
            lines.push(entry.file_name().to_string_lossy().into_owned());
        }
        Ok(ToolResponse {
            id: call.id.clone(),
            name: call.name.clone(),
            content: lines.join("\n"),
            success: true,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[tokio::test]
    async fn reads_only_inside_root_and_truncates() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("a.txt"), "hello").unwrap();
        let t = ReadFileTool::new(d.path());
        let c = ToolCall {
            id: None,
            name: "read_file".into(),
            arguments: r#"{"path":"a.txt"}"#.into(),
        };
        assert_eq!(
            t.execute(&c, CancellationToken::new())
                .await
                .unwrap()
                .content,
            "hello"
        );
        let outside = ToolCall {
            id: None,
            name: "read_file".into(),
            arguments: r#"{"path":"../nope"}"#.into(),
        };
        assert!(matches!(
            t.execute(&outside, CancellationToken::new()).await,
            Err(ToolError::Denied(_))
        ));
    }
    #[tokio::test]
    async fn lists_entries() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("a"), "x").unwrap();
        let t = ListDirTool::new(d.path());
        let c = ToolCall {
            id: None,
            name: "list_dir".into(),
            arguments: r#"{"path":"."}"#.into(),
        };
        assert_eq!(
            t.execute(&c, CancellationToken::new())
                .await
                .unwrap()
                .content,
            "a"
        );
    }
}
