//! Session save/resume for crash recovery.
//!
//! Sessions are saved as JSON files in a configurable directory.
//! Progress is tracked by command index, allowing resumption from any point.

use crate::painting::{PaintPlan, ScreenRect};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A resumable painting session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub plan: PaintPlan,
    pub progress: usize,
    pub canvas_region: ScreenRect,
    pub image_path: String,
    pub img_width: u32,
    pub img_height: u32,
}

impl Session {
    pub fn new(
        plan: PaintPlan,
        canvas_region: ScreenRect,
        image_path: String,
        img_width: u32,
        img_height: u32,
    ) -> Self {
        Self {
            plan,
            progress: 0,
            canvas_region,
            image_path,
            img_width,
            img_height,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.progress >= self.plan.commands.len()
    }

    pub fn progress_percent(&self) -> f64 {
        if self.plan.commands.is_empty() {
            return 100.0;
        }
        self.progress as f64 / self.plan.commands.len() as f64 * 100.0
    }

    pub fn remaining_commands(&self) -> usize {
        self.plan.commands.len().saturating_sub(self.progress)
    }

    /// Save session to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create session directory: {}", e))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize session: {}", e))?;
        std::fs::write(path, json)
            .map_err(|e| format!("Failed to write session file: {}", e))?;
        Ok(())
    }

    /// Load a session from a JSON file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read session file: {}", e))?;
        serde_json::from_str(&json)
            .map_err(|e| format!("Failed to deserialize session: {}", e))
    }

    /// Get the default sessions directory (~/.rustbrush/sessions/).
    pub fn default_sessions_dir() -> PathBuf {
        dirs_or_fallback().join("sessions")
    }

    /// Generate a session file path based on image name and timestamp.
    pub fn generate_path(image_path: &str) -> PathBuf {
        let stem = Path::new(image_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self::default_sessions_dir().join(format!("{}_{}.json", stem, timestamp))
    }
}

fn dirs_or_fallback() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
    {
        PathBuf::from(home).join(".rustbrush")
    } else {
        PathBuf::from(".rustbrush")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::painting::{PaintCommand, PaintPlan, PlanMetadata};

    fn test_session() -> Session {
        Session::new(
            PaintPlan {
                commands: vec![
                    PaintCommand::MoveTo { x: 10, y: 20 },
                    PaintCommand::Click,
                    PaintCommand::MoveTo { x: 30, y: 40 },
                    PaintCommand::Click,
                ],
                metadata: PlanMetadata {
                    total_pixels: 2,
                    total_colors: 1,
                    total_commands: 4,
                    strategy_name: "test".to_string(),
                },
            },
            ScreenRect { x: 0, y: 0, width: 100, height: 100 },
            "test.png".to_string(),
            256,
            256,
        )
    }

    #[test]
    fn test_session_progress() {
        let mut session = test_session();
        assert_eq!(session.progress_percent(), 0.0);
        assert!(!session.is_complete());
        assert_eq!(session.remaining_commands(), 4);

        session.progress = 2;
        assert_eq!(session.progress_percent(), 50.0);
        assert_eq!(session.remaining_commands(), 2);

        session.progress = 4;
        assert!(session.is_complete());
        assert_eq!(session.progress_percent(), 100.0);
    }

    #[test]
    fn test_session_save_load_roundtrip() {
        let session = test_session();
        let tmp_dir = std::env::temp_dir().join("rustbrush_test_sessions");
        let path = tmp_dir.join("test_session.json");

        session.save(&path).unwrap();
        let loaded = Session::load(&path).unwrap();

        assert_eq!(loaded.progress, session.progress);
        assert_eq!(loaded.plan.commands.len(), session.plan.commands.len());
        assert_eq!(loaded.image_path, session.image_path);
        assert_eq!(loaded.img_width, session.img_width);

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&tmp_dir);
    }

    #[test]
    fn test_session_load_nonexistent() {
        let result = Session::load(Path::new("/nonexistent/path.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_path_contains_image_name() {
        let path = Session::generate_path("my_painting.png");
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert!(filename.starts_with("my_painting_"));
        assert!(filename.ends_with(".json"));
    }
}
