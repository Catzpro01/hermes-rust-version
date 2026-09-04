use super::{readonly::safe_path, Confirmation, Tool, ToolCall, ToolError, ToolResponse};
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;
use tokio_util::sync::CancellationToken;

pub struct WriteFileTool<C> {
    root: PathBuf,
    confirm: C,
}
impl<C> WriteFileTool<C> {
    pub fn new(root: impl Into<PathBuf>, confirm: C) -> Self {
        Self {
            root: root.into(),
            confirm,
        }
    }
}
fn args(input: &str) -> Result<(String, String), ToolError> {
    let value: Value = serde_json::from_str(input)
        .map_err(|_| ToolError::Failed("write_file expects JSON {path, content}".into()))?;
    let path = value
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::Failed("missing string argument: path".into()))?
        .to_owned();
    let content = value
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::Failed("missing string argument: content".into()))?
        .to_owned();
    Ok((path, content))
}
#[async_trait]
impl<C: Confirmation> Tool for WriteFileTool<C> {
    fn name(&self) -> &str {
        "write_file"
    }
    fn description(&self) -> &str {
        "Write a file under the tool root after explicit confirmation."
    }
    async fn execute(
        &self,
        call: &ToolCall,
        cancel: CancellationToken,
    ) -> Result<ToolResponse, ToolError> {
        let (requested, content) = args(&call.arguments)?;
        let path = safe_path(&self.root, &requested).await.or_else(|e| {
            if matches!(e, ToolError::Failed(_)) {
                Ok(self.root.join(&requested))
            } else {
                Err(e)
            }
        })?;
        if cancel.is_cancelled() {
            return Err(ToolError::Cancelled);
        };
        let prompt = format!(
            "Write to {} ({} bytes)? [y/N]",
            path.display(),
            content.len()
        );
        if !self.confirm.confirm(&prompt).await {
            return Err(ToolError::Denied("confirmation declined".into()));
        }
        tokio::select! { _=cancel.cancelled()=>Err(ToolError::Cancelled), result=fs::write(&path,content.as_bytes())=>{ result.map_err(|e|ToolError::Failed(e.to_string()))?; Ok(ToolResponse{id:call.id.clone(),name:call.name.clone(),content:format!("wrote {}",path.display()),success:true}) } }
    }
}
