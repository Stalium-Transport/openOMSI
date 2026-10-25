//! Stalium: the BCS directory (`bcs.txt` beside `server.cfg`).
//!
//! The Stalium game, started by BCS for a trip, asks this server where to go:
//! `GET /stalium/bcs?carte=maps/st-servan/global.cfg&nom=<player>` on its web port. The answer
//! is `join <port>` (the server of that map, on this same machine), `join <host:port>` or
//! `solo <why>`. A player who is not in `membres.txt` (when it lists names) drives alone.
//!
//! `bcs.txt`, one map a line (`#` starts a comment), read again on every question:
//!
//! ```text
//! maps/st-servan/global.cfg      = 27017
//! maps/berlin-spandau/global.cfg = 27021
//! maps/autre/global.cfg          = 1.2.3.4:27015
//! ```
//!
//! No `bcs.txt`: the directory is off (404).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The file's name, beside `server.cfg`.
pub const FILE_NAME: &str = "bcs.txt";

static FILE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// `maps/St-Servan/global.cfg`, `.\Maps\st-servan\global.cfg` … as one name.
pub fn canonical_map(map: &str) -> String {
    let m = map.trim().replace('\\', "/").to_lowercase();
    let m = m.trim_start_matches("./");
    match m.find("maps/") {
        Some(i) => m[i..].to_string(),
        None => m.to_string(),
    }
}

/// What a `bcs.txt` says: (map, port or host:port).
pub fn parse(text: &str) -> Vec<(String, String)> {
    text.lines()
        .map(|l| l.trim().trim_start_matches('\u{feff}'))
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .map(|(m, t)| (canonical_map(m), t.trim().to_string()))
        .filter(|(m, t)| !m.is_empty() && !t.is_empty())
        .collect()
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or(""), 16) {
                    Ok(v) => {
                        out.push(v);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn query_value(query: &str, key: &str) -> String {
    query.split('&').filter_map(|kv| kv.split_once('=')).find(|(k, _)| *k == key).map(|(_, v)| decode(v)).unwrap_or_default()
}

/// The answer to a question (without the file: None).
pub fn route(routes: &[(String, String)], map: &str, member: bool) -> String {
    if !member {
        return "solo pas membre".into();
    }
    let m = canonical_map(map);
    match routes.iter().find(|(k, _)| *k == m) {
        Some((_, t)) => format!("join {t}"),
        None => "solo pas de serveur pour cette carte".into(),
    }
}

fn answer(path: &str) -> Option<(&'static str, String)> {
    let (route_path, query) = path.split_once('?').unwrap_or((path, ""));
    if route_path != "/stalium/bcs" {
        return None;
    }
    let file = FILE.lock().unwrap_or_else(|e| e.into_inner()).clone()?;
    let bytes = std::fs::read(&file).ok()?;
    let text = String::from_utf8(bytes.clone()).unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect());
    let map = query_value(query, "carte");
    let name = query_value(query, "nom");
    let a = route(&parse(&text), &map, crate::members::allowed(&name));
    log::info!("server: BCS : '{}' sur {} -> {a}", name.trim(), canonical_map(&map));
    Some(("200 OK", a))
}

/// Answer BCS's questions from `bcs.txt` in the folder of `server_cfg`.
pub fn watch(server_cfg: &Path) {
    let dir = server_cfg.parent().unwrap_or(Path::new("."));
    *FILE.lock().unwrap_or_else(|e| e.into_inner()) = Some(dir.join(FILE_NAME));
    omsi_net::ws::set_extra_route(answer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes() {
        let r = parse("# x\r\nmaps/St-Servan/global.cfg = 27017\r\nMaps\\Berlin-Spandau\\global.cfg=1.2.3.4:27021\r\nbad line\r\n");
        assert_eq!(r.len(), 2);
        assert_eq!(route(&r, "maps/st-servan/global.cfg", true), "join 27017");
        assert_eq!(route(&r, "Maps/Berlin-Spandau/global.cfg", true), "join 1.2.3.4:27021");
        assert!(route(&r, "maps/grundorf/global.cfg", true).starts_with("solo"));
        assert!(route(&r, "maps/st-servan/global.cfg", false).starts_with("solo"));
        assert_eq!(query_value("carte=maps%2FSt-Servan%2Fglobal.cfg&nom=Jean+Dupont", "nom"), "Jean Dupont");
        assert_eq!(query_value("carte=maps%2FSt-Servan%2Fglobal.cfg&nom=x", "carte"), "maps/St-Servan/global.cfg");
        assert_eq!(decode("a%"), "a%");
    }
}
