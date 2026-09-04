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
        .or_else(|_| {
            let mut path = None;
            let mut content = None;
            for line in input.lines() {
                if let Some(v) = line.strip_prefix("path:") {
                    path = Some(v.trim().to_owned());
                }
                if let Some(v) = line.strip_prefix("content:") {
                    content = Some(v.trim_start().to_owned());
                }
            }
            match (path, content) {
                (Some(path), Some(content)) => {
                    Ok(serde_json::json!({"path":path,"content":content}))
                }
                _ => Err(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "invalid write arguments",
                ))),
            }
        })
        .map_err(|_| {
            ToolError::Failed("write_file expects JSON {path, content} or path/content text".into())
        })?;
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

#[cfg(test)]
mod tests {
    use super::*;
    struct Confirm(bool);
    #[async_trait]
    impl Confirmation for Confirm {
        async fn confirm(&self, _: &str) -> bool {
            self.0
        }
    }
    fn call(args: &str) -> ToolCall {
        ToolCall {
            id: Some("x".into()),
            name: "write_file".into(),
            arguments: args.into(),
        }
    }
    #[tokio::test]
    async fn confirmed_and_denied() {
        let d = tempfile::tempdir().unwrap();
        let t = WriteFileTool::new(d.path(), Confirm(true));
        t.execute(
            &call(r#"{"path":"a.txt","content":"ok"}"#),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(d.path().join("a.txt")).unwrap(),
            "ok"
        );
        let t = WriteFileTool::new(d.path(), Confirm(false));
        assert!(matches!(
            t.execute(
                &call(r#"{"path":"b.txt","content":"no"}"#),
                CancellationToken::new()
            )
            .await,
            Err(ToolError::Denied(_))
        ));
        assert!(!d.path().join("b.txt").exists());
    }
    #[tokio::test]
    async fn traversal_and_text_args_rejected_or_supported() {
        let d = tempfile::tempdir().unwrap();
        let t = WriteFileTool::new(d.path(), Confirm(true));
        assert!(t
            .execute(
                &call("path: text.txt\ncontent: hello"),
                CancellationToken::new()
            )
            .await
            .is_ok());
        assert!(matches!(
            t.execute(
                &call(r#"{"path":"../escape","content":"x"}"#),
                CancellationToken::new()
            )
            .await,
            Err(ToolError::Failed(_))
        ));
    }
}
