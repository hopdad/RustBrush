//! Session save/resume for crash recovery.

use crate::painting::{PaintPlan, ScreenRect};
use serde::{Deserialize, Serialize};

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
}
