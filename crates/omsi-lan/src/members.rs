//! Stalium: a server kept for its members (`membres.txt` beside `server.cfg`).
//!
//! The file lists the player names allowed on the server, one a line (case and spaces at the
//! ends do not count; `#` starts a comment). A line `message: …` sets what a player who is
//! not on the list reads when the server sends them away. The file is read again within a few
//! seconds of a change, so members are added or taken off while the server runs. No file (or
//! one without any name): every player may join, as on an unmodified server.
//!
//! The check is the name the player's game gives: it keeps strangers out, not someone who
//! takes a member's name on purpose.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

/// The file's name, beside `server.cfg`.
pub const FILE_NAME: &str = "membres.txt";
const DEFAULT_MESSAGE: &str = "Serveur reserve aux membres Stalium Transport : inscris-toi sur notre Discord";

#[derive(Default)]
struct Members {
    path: Option<PathBuf>,
    /// The file's change time when it was read (None: not read yet, or no file).
    stamp: Option<SystemTime>,
    checked: Option<Instant>,
    names: Vec<String>,
    message: String,
}

static MEMBERS: Mutex<Option<Members>> = Mutex::new(None);

/// A name as the list compares it.
fn norm(name: &str) -> String {
    name.trim().to_lowercase()
}

/// What a `membres.txt` says: the names and the message.
pub fn parse(text: &str) -> (Vec<String>, String) {
    let mut names = Vec::new();
    let mut message = String::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(m) = line.strip_prefix("message:") {
            message = m.trim().to_string();
            continue;
        }
        let n = norm(line);
        if !n.is_empty() && !names.contains(&n) {
            names.push(n);
        }
    }
    (names, message)
}

/// Keep the server to the members listed in `membres.txt` in the folder of `server_cfg`.
pub fn watch(server_cfg: &Path) {
    let dir = server_cfg.parent().unwrap_or(Path::new("."));
    let path = dir.join(FILE_NAME);
    let mut g = MEMBERS.lock().unwrap_or_else(|e| e.into_inner());
    *g = Some(Members { path: Some(path), ..Default::default() });
}

/// Read the file again when it changed (looked at every few seconds at most).
fn refresh(m: &mut Members) {
    if m.checked.is_some_and(|t| t.elapsed() < Duration::from_secs(3)) {
        return;
    }
    m.checked = Some(Instant::now());
    let Some(path) = m.path.clone() else { return };
    let stamp = std::fs::metadata(&path).and_then(|md| md.modified()).ok();
    if stamp == m.stamp {
        return;
    }
    m.stamp = stamp;
    match stamp.and_then(|_| std::fs::read(&path).ok()) {
        Some(bytes) => {
            let text = String::from_utf8(bytes.clone()).unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect());
            let (names, message) = parse(&text);
            log::info!("server: {} : {} membre(s) autorise(s)", path.display(), names.len());
            m.names = names;
            m.message = message;
        }
        None => {
            if !m.names.is_empty() {
                log::info!("server: {} n'existe plus : le serveur est ouvert a tous", path.display());
            }
            m.names.clear();
            m.message.clear();
        }
    }
}

/// Whether `name` may be on the server (no list: everybody).
pub fn allowed(name: &str) -> bool {
    let mut g = MEMBERS.lock().unwrap_or_else(|e| e.into_inner());
    let Some(m) = g.as_mut() else { return true };
    refresh(m);
    m.names.is_empty() || m.names.contains(&norm(name))
}

/// Send away every player who is not a member (called about once a second by the server).
pub fn enforce(lan: &mut omsi_net::LanSession) {
    let message = {
        let mut g = MEMBERS.lock().unwrap_or_else(|e| e.into_inner());
        let Some(m) = g.as_mut() else { return };
        refresh(m);
        if m.names.is_empty() {
            return;
        }
        if m.message.is_empty() { DEFAULT_MESSAGE.to_string() } else { m.message.clone() }
    };
    let out: Vec<(u32, String)> = lan.peers().filter(|p| p.has_info && !allowed(&p.pose.name)).map(|p| (p.pose.id, p.pose.name.clone())).collect();
    for (id, name) in out {
        log::warn!("server: player {id} '{name}' n'est pas dans {FILE_NAME} : renvoye");
        lan.kick(id, &message, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_comments_and_message() {
        let (names, message) = parse("\u{feff}# membres Stalium\nMichael\n  alex  \nMICHAEL\n\nmessage: Rejoins le Discord !\n");
        assert_eq!(names, vec!["michael".to_string(), "alex".to_string()]);
        assert_eq!(message, "Rejoins le Discord !");
    }

    #[test]
    fn empty_file_is_open() {
        let (names, _) = parse("# personne\n\n");
        assert!(names.is_empty());
    }
}
