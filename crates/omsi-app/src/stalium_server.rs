//! Stalium: what a dedicated server's `server.cfg` says of its timetable buses at the stops.
//!
//! ```text
//! # an early bus waits for its departure only at the stops the timetable times (1), or at
//! # every stop it serves, as OMSI does (0)
//! ai_wait_timed_stops_only = 1
//! # the longest an early bus stands at a stop, its trip's last stop too (seconds; 0: as
//! # OMSI, up to 30 minutes and at the last stop however long)
//! ai_stop_wait_max = 120
//! # a timetable bus standing this long (seconds) with nothing holding it - no light, no
//! # queue, no passengers, no time of its own at a stop - is taken off the road and the next
//! # of its line goes on (0: never)
//! ai_stuck_remove = 300
//! ```
//!
//! Without the keys a Stalium server takes these values: buses standing for many minutes at a
//! stop on the street held the lane behind them and jammed the traffic. A game that is no
//! server keeps the player's own settings.

use std::path::Path;
use std::sync::OnceLock;

/// The values without the keys.
pub const TIMED_ONLY_DEFAULT: bool = true;
pub const WAIT_MAX_DEFAULT: f64 = 120.0;
pub const STUCK_REMOVE_DEFAULT: f32 = 300.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StopWaits {
    pub timed_only: bool,
    /// None: no cap (OMSI's waits).
    pub wait_max: Option<f64>,
    /// Seconds a timetable bus stands with nothing holding it before it is taken off (None:
    /// never).
    pub stuck_remove: Option<f32>,
}

static WAITS: OnceLock<StopWaits> = OnceLock::new();

/// What a `server.cfg`'s text says.
pub fn parse(text: &str) -> StopWaits {
    let mut w = StopWaits { timed_only: TIMED_ONLY_DEFAULT, wait_max: Some(WAIT_MAX_DEFAULT), stuck_remove: Some(STUCK_REMOVE_DEFAULT) };
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('#') {
            continue;
        }
        let Some((k, v)) = l.split_once('=') else { continue };
        let v = v.trim();
        match k.trim().to_ascii_lowercase().as_str() {
            "ai_wait_timed_stops_only" if !v.is_empty() => w.timed_only = matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
            "ai_stop_wait_max" => {
                if let Ok(s) = v.parse::<f64>() {
                    w.wait_max = (s.is_finite() && s > 0.0).then_some(s.min(3600.0));
                }
            }
            "ai_stuck_remove" => {
                if let Ok(s) = v.parse::<f32>() {
                    // (at least a minute: a bus may stand a while for a reason the test misses)
                    w.stuck_remove = (s.is_finite() && s > 0.0).then_some(s.clamp(60.0, 7200.0));
                }
            }
            _ => {}
        }
    }
    w
}

/// Read once by the server's start (`server::prepare`).
pub fn load(server_cfg: &Path) {
    let w = parse(&std::fs::read_to_string(server_cfg).unwrap_or_default());
    log::info!(
        "server: timetable buses wait for their departure {}, {}",
        if w.timed_only { "only at the stops the timetable times" } else { "at every stop, as in OMSI" },
        match w.wait_max {
            Some(s) => format!("{s:.0} s at most at a stop"),
            None => "as long as the timetable says".into(),
        }
    );
    log::info!(
        "server: a timetable bus standing with nothing holding it is {}",
        match w.stuck_remove {
            Some(s) => format!("taken off after {s:.0} s (ai_stuck_remove)"),
            None => "never taken off (ai_stuck_remove = 0)".into(),
        }
    );
    let _ = WAITS.set(w);
}

/// The server's waits (None: not a server).
pub fn waits() -> Option<StopWaits> {
    WAITS.get().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_defaults() {
        assert_eq!(parse(""), StopWaits { timed_only: true, wait_max: Some(120.0), stuck_remove: Some(300.0) });
        assert_eq!(parse("ai_wait_timed_stops_only = 0\nai_stop_wait_max = 0\nai_stuck_remove = 0\n"), StopWaits { timed_only: false, wait_max: None, stuck_remove: None });
        assert_eq!(parse("AI_STOP_WAIT_MAX=300\n# ai_wait_timed_stops_only = 0\n"), StopWaits { timed_only: true, wait_max: Some(300.0), stuck_remove: Some(300.0) });
        assert_eq!(parse("ai_stuck_remove = 600\n").stuck_remove, Some(600.0));
        assert_eq!(parse("ai_stuck_remove = 5\n").stuck_remove, Some(60.0), "a minute at least");
        assert_eq!(parse("ai_stop_wait_max = abc\n").wait_max, Some(120.0));
    }
}
