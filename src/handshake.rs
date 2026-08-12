use crate::api::ReplayClient;
use crate::detect;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum WatchOutcome {
    Ready { pid: u32 },
    Crashed { log: String },
    Timeout { log: String },
}

/// After LCU returns 204, wait until the game process is up *and* the Replay API answers.
/// If the process appears then dies, treat it as a crash and attach the latest r3dlog tail.
pub fn await_replay(timeout: Duration) -> WatchOutcome {
    let client = match ReplayClient::new() {
        Ok(c) => c,
        Err(e) => {
            return WatchOutcome::Timeout {
                log: format!("http client: {e}"),
            };
        }
    };
    let start = Instant::now();
    let mut saw_pid = false;
    let mut last_pid = 0u32;
    while start.elapsed() < timeout {
        let pid = detect::game_pid();
        if let Some(pid) = pid {
            saw_pid = true;
            last_pid = pid;
            if client.game().is_ok() {
                return WatchOutcome::Ready { pid };
            }
        } else if saw_pid {
            return WatchOutcome::Crashed {
                log: detect::latest_r3dlog_tail(40),
            };
        }
        thread::sleep(Duration::from_millis(400));
    }
    if saw_pid && detect::game_pid().is_none() {
        return WatchOutcome::Crashed {
            log: detect::latest_r3dlog_tail(40),
        };
    }
    WatchOutcome::Timeout {
        log: format!(
            "waited {:.0}s (saw_game={saw_pid} last_pid={last_pid})\n{}",
            timeout.as_secs_f32(),
            detect::latest_r3dlog_tail(24)
        ),
    }
}

pub fn format_outcome(out: &WatchOutcome) -> String {
    match out {
        WatchOutcome::Ready { pid } => format!("Replay API up (game pid {pid})."),
        WatchOutcome::Crashed { log } => {
            format!("Game launched then died before the Replay API came up.\n{log}")
        }
        WatchOutcome::Timeout { log } => {
            format!("Timed out waiting for Replay API.\n{log}")
        }
    }
}
