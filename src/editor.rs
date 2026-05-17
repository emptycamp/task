use crate::error::{Error, Result};
use std::path::Path;
use std::process::Command;

pub trait EditorLauncher {
    fn launch(&self, path: &Path) -> Result<()>;
}

pub struct SystemEditor;

impl EditorLauncher for SystemEditor {
    fn launch(&self, path: &Path) -> Result<()> {
        let editor = std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| {
                if cfg!(windows) {
                    "notepad".to_string()
                } else {
                    "vi".to_string()
                }
            });

        let status = Command::new(&editor)
            .arg(path)
            .status()
            .map_err(|e| Error::EditorError(format!("failed to launch '{editor}': {e}")))?;

        if !status.success() {
            return Err(Error::EditorError(format!(
                "editor '{editor}' exited with status {status}"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn system_editor_reads_editor_env_var() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        std::fs::write(&file, "content").unwrap();

        // Use "true" (a no-op command) as the editor
        env::set_var("EDITOR", "true");
        let result = SystemEditor.launch(&file);
        env::remove_var("EDITOR");
        assert!(result.is_ok());
    }

    #[test]
    fn system_editor_falls_back_to_visual_when_editor_unset() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        std::fs::write(&file, "content").unwrap();

        env::remove_var("EDITOR");
        env::set_var("VISUAL", "true");
        let result = SystemEditor.launch(&file);
        env::remove_var("VISUAL");
        assert!(result.is_ok());
    }
}
