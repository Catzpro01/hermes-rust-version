use super::{Confirmation, Tool, ToolCall, ToolError, ToolResponse};
use async_trait::async_trait;
use serde_json::Value;
use std::{
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
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
pub fn safe_write_path(root: &Path, requested: &str) -> Result<PathBuf, ToolError> {
    let req = Path::new(requested);
    if req.is_absolute() {
        return Err(ToolError::Failed("Absolute paths are not allowed".into()));
    }
    for c in req.components() {
        if !matches!(c, Component::Normal(_) | Component::CurDir) {
            return Err(ToolError::Failed(
                "Path traversal or invalid component".into(),
            ));
        }
    }
    let root = root
        .canonicalize()
        .map_err(|e| ToolError::Failed(format!("Invalid root: {e}")))?;
    let target = root.join(req);
    if target.exists() {
        let canon = target
            .canonicalize()
            .map_err(|e| ToolError::Failed(e.to_string()))?;
        if !canon.starts_with(&root) {
            return Err(ToolError::Failed("Target escapes root jail".into()));
        }
    }
    Ok(target)
}
fn args(input: &str) -> Result<(String, String), ToolError> {
    let v: Value = serde_json::from_str(input)
        .map_err(|_| ToolError::Failed("write_file expects JSON {path, content}".into()))?;
    Ok((
        v.get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Failed("missing path".into()))?
            .into(),
        v.get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Failed("missing content".into()))?
            .into(),
    ))
}
#[async_trait]
impl<C: Confirmation> Tool for WriteFileTool<C> {
    fn name(&self) -> &str {
        "write_file"
    }
    fn description(&self) -> &str {
        "Write a file after confirmation."
    }
    async fn execute(
        &self,
        call: &ToolCall,
        cancel: CancellationToken,
    ) -> Result<ToolResponse, ToolError> {
        let (requested, content) = args(&call.arguments)?;
        let target = safe_write_path(&self.root, &requested)?;
        if cancel.is_cancelled() {
            return Err(ToolError::Cancelled);
        };
        if !self
            .confirm
            .confirm(&format!(
                "Write to {} ({} bytes)? [y/N]",
                target.display(),
                content.len()
            ))
            .await
        {
            return Err(ToolError::Denied("confirmation declined".into()));
        }
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tmp = target.with_file_name(format!(
            ".{}.tmp.{}",
            target.file_name().unwrap_or_default().to_string_lossy(),
            nonce
        ));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| ToolError::Failed(e.to_string()))?;
        }
        tokio::select! {_=cancel.cancelled()=>Err(ToolError::Cancelled),r=fs::write(&tmp,content.as_bytes())=>{r.map_err(|e|ToolError::Failed(e.to_string()))?;if cancel.is_cancelled(){let _=fs::remove_file(&tmp).await;return Err(ToolError::Cancelled)};fs::rename(&tmp,&target).await.map_err(|e|ToolError::Failed(e.to_string()))?;Ok(ToolResponse{id:call.id.clone(),name:call.name.clone(),content:format!("wrote {} bytes",content.len()),success:true})}}
    }
}
