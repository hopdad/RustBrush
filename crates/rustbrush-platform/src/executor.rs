//! Paint plan executor - executes PaintCommands via an InputDriver.
//!
//! Supports pause/resume/cancel via PaintControl, progress callbacks,
//! and session save for crash recovery.

use crate::hotkey::PaintControl;
use crate::input::InputDriver;
use rustbrush_core::painting::{PaintCommand, PaintPlan};
use rustbrush_core::session::Session;
use std::sync::mpsc;
use std::time::Duration;

/// Progress update sent from executor to GUI.
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub commands_executed: usize,
    pub total_commands: usize,
    pub percent: f64,
    pub current_color: Option<String>,
    pub paused: bool,
}

/// Result of plan execution.
pub enum ExecutionResult {
    Completed {
        commands_executed: usize,
    },
    Cancelled {
        commands_executed: usize,
        total_commands: usize,
    },
    Error {
        commands_executed: usize,
        error: String,
    },
}

/// Configuration for the executor.
pub struct ExecutorConfig {
    /// How often to save progress (every N commands). 0 = disabled.
    pub save_interval: usize,
    /// Path to save session file for resume. None = no saving.
    pub session_path: Option<std::path::PathBuf>,
    /// How often to report progress (every N commands).
    pub progress_interval: usize,
    /// Optional channel to send progress updates to the GUI.
    pub progress_tx: Option<mpsc::Sender<ProgressUpdate>>,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            save_interval: 500,
            session_path: None,
            progress_interval: 500,
            progress_tx: None,
        }
    }
}

/// Execute a paint plan using an InputDriver, with pause/cancel support.
///
/// Starts from `start_index` to support resuming interrupted sessions.
pub fn execute_plan(
    plan: &PaintPlan,
    input: &mut dyn InputDriver,
    control: &PaintControl,
    config: &ExecutorConfig,
    mut session: Option<&mut Session>,
    start_index: usize,
) -> ExecutionResult {
    let total = plan.commands.len();
    let mut executed = 0usize;
    let mut current_color: Option<String> = None;

    for i in start_index..total {
        // Check pause/cancel
        let is_paused = control.paused.load(portable_atomic::Ordering::Relaxed);
        if !control.check() {
            // Send cancelled update
            if let Some(ref tx) = config.progress_tx {
                let _ = tx.send(ProgressUpdate {
                    commands_executed: executed,
                    total_commands: total,
                    percent: executed as f64 / total as f64 * 100.0,
                    current_color: current_color.clone(),
                    paused: false,
                });
            }
            if let (Some(session), Some(path)) = (&session, &config.session_path) {
                let _ = session.save(path);
            }
            return ExecutionResult::Cancelled {
                commands_executed: executed,
                total_commands: total,
            };
        }

        // Track current color
        if let PaintCommand::SelectColorByHex { hex } = &plan.commands[i] {
            current_color = Some(hex.clone());
        }

        let cmd = &plan.commands[i];
        if let Err(e) = execute_command(cmd, input) {
            return ExecutionResult::Error {
                commands_executed: executed,
                error: e,
            };
        }

        executed += 1;

        // Update session progress
        if let Some(ref mut session) = session {
            session.progress = i + 1;
        }

        // Periodic save
        if config.save_interval > 0
            && executed % config.save_interval == 0
        {
            if let (Some(session), Some(path)) = (&session, &config.session_path) {
                let _ = session.save(path);
            }
        }

        // Progress reporting
        if config.progress_interval > 0 && executed % config.progress_interval == 0 {
            let pct = (i + 1) as f64 / total as f64 * 100.0;
            println!("  Progress: {}/{} ({:.1}%)", i + 1, total, pct);

            // Send progress via channel
            if let Some(ref tx) = config.progress_tx {
                let _ = tx.send(ProgressUpdate {
                    commands_executed: i + 1,
                    total_commands: total,
                    percent: pct,
                    current_color: current_color.clone(),
                    paused: is_paused,
                });
            }
        }
    }

    // Final save on completion
    if let (Some(session), Some(path)) = (session, &config.session_path) {
        session.progress = total;
        let _ = session.save(path);
    }

    // Send final progress
    if let Some(ref tx) = config.progress_tx {
        let _ = tx.send(ProgressUpdate {
            commands_executed: total,
            total_commands: total,
            percent: 100.0,
            current_color,
            paused: false,
        });
    }

    ExecutionResult::Completed {
        commands_executed: executed,
    }
}

/// Execute a single PaintCommand.
fn execute_command(cmd: &PaintCommand, input: &mut dyn InputDriver) -> Result<(), String> {
    match cmd {
        PaintCommand::MoveTo { x, y } => input.move_to(*x, *y),
        PaintCommand::Click => input.click(),
        PaintCommand::ShiftClick => input.shift_click(),
        PaintCommand::SelectColorByHex { hex } => {
            input.click()?;
            std::thread::sleep(Duration::from_millis(30));
            input.select_all()?;
            std::thread::sleep(Duration::from_millis(20));
            input.type_text(hex)?;
            std::thread::sleep(Duration::from_millis(20));
            input.press_key_return()?;
            std::thread::sleep(Duration::from_millis(30));
            Ok(())
        }
        PaintCommand::SelectColorByClick { x, y } => {
            input.move_to(*x, *y)?;
            input.click()
        }
        PaintCommand::Delay { ms } => {
            std::thread::sleep(Duration::from_millis(*ms as u64));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::DryRunInput;
    use rustbrush_core::painting::PlanMetadata;

    fn test_plan() -> PaintPlan {
        PaintPlan {
            commands: vec![
                PaintCommand::SelectColorByHex { hex: "FF0000".to_string() },
                PaintCommand::Delay { ms: 0 },
                PaintCommand::MoveTo { x: 10, y: 20 },
                PaintCommand::Click,
                PaintCommand::MoveTo { x: 30, y: 20 },
                PaintCommand::ShiftClick,
                PaintCommand::MoveTo { x: 50, y: 40 },
                PaintCommand::Click,
            ],
            metadata: PlanMetadata {
                total_pixels: 3,
                total_colors: 1,
                total_commands: 8,
                strategy_name: "test".to_string(),
            },
        }
    }

    #[test]
    fn test_execute_full_plan() {
        let plan = test_plan();
        let mut input = DryRunInput::new();
        let control = PaintControl::new();
        let config = ExecutorConfig {
            save_interval: 0,
            session_path: None,
            progress_interval: 0,
            progress_tx: None,
        };

        let result = execute_plan(&plan, &mut input, &control, &config, None, 0);
        match result {
            ExecutionResult::Completed { commands_executed } => {
                assert_eq!(commands_executed, 8);
            }
            _ => panic!("Expected Completed"),
        }

        // Verify the commands were executed
        assert!(input.log.iter().any(|s| s == "SHIFT_CLICK"));
        assert!(input.log.iter().any(|s| s.starts_with("TYPE")));
    }

    #[test]
    fn test_execute_plan_resume_from_index() {
        let plan = test_plan();
        let mut input = DryRunInput::new();
        let control = PaintControl::new();
        let config = ExecutorConfig {
            save_interval: 0,
            session_path: None,
            progress_interval: 0,
            progress_tx: None,
        };

        // Resume from command index 4
        let result = execute_plan(&plan, &mut input, &control, &config, None, 4);
        match result {
            ExecutionResult::Completed { commands_executed } => {
                assert_eq!(commands_executed, 4); // only commands 4..8
            }
            _ => panic!("Expected Completed"),
        }
    }

    #[test]
    fn test_execute_plan_cancel() {
        let plan = test_plan();
        let mut input = DryRunInput::new();
        let control = PaintControl::new();
        // Pre-cancel
        control.cancelled.store(true, portable_atomic::Ordering::Relaxed);

        let config = ExecutorConfig::default();
        let result = execute_plan(&plan, &mut input, &control, &config, None, 0);
        match result {
            ExecutionResult::Cancelled { commands_executed, .. } => {
                assert_eq!(commands_executed, 0);
            }
            _ => panic!("Expected Cancelled"),
        }
    }
}
