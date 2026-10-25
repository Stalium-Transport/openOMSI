//! Stalium Transport: a BCS (Busbetrieb-Simulator) trip joins the Stalium server by itself,
//! with nothing to set in BCS (it only picks openOMSI's folder).
//!
//! BCS starts the game with its own personnel file (`--driver …\\.openomsi\\bbs\\bbs.odr`),
//! `--no-menu` and the trip's situation. On such a start, with no LAN option of its own, the
//! game asks the Stalium directory (`/stalium/bcs` on a Stalium server, see
//! `omsi_lan::bcs`) where this player goes on this map: `join …` gets `--lan-join`, anything
//! else - not a member, a map without a server, no answer - is driven alone, as before. The
//! maps, the ports and the members are the server's files (`bcs.txt`, `membres.txt`): they
//! change with no new release. Only the directory's address is here, kept scrambled.

use crate::cli::Args;
use std::time::Duration;

const KEY: &[u8] = b"Stalium-BCS-2026";
/// The directory's `host:port` (a Stalium server's web port), XOR [`KEY`]
/// (written by scripts/stalium/servers.py).
const DIRECTORY: &[u8] = &[
    0x6b, 0x46, 0x4f, 0x5d, 0x5f, 0x40, 0x43, 0x1b, 0x75, 0x6d, 0x62, 0x1f, 0x05, 0x0a, 0x00, 0x01,
    0x63, 0x46, 0x56
];

fn directory() -> String {
    let plain: Vec<u8> = DIRECTORY.iter().enumerate().map(|(i, b)| b ^ KEY[i % KEY.len()]).collect();
    String::from_utf8_lossy(&plain).trim().to_string()
}

/// A start by BCS: its own personnel file `bbs\bbs.odr`.
fn started_by_bcs(args: &Args) -> bool {
    args.driver.as_deref().map(|d| d.replace('\\', "/").to_lowercase().ends_with("bbs/bbs.odr")).unwrap_or(false)
}

fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The directory's answer as a `--lan-join` target: `join 27017` is that port on the
/// directory's own machine, `join host:port` a server elsewhere; None: drive alone.
fn target(answer: &str, directory: &str) -> Option<String> {
    let t = answer.trim().strip_prefix("join ")?.trim();
    if t.is_empty() {
        return None;
    }
    if t.bytes().all(|b| b.is_ascii_digit()) {
        let host = directory.rsplit_once(':').map(|(h, _)| h).unwrap_or(directory);
        return Some(format!("{host}:{t}"));
    }
    Some(t.to_string())
}

fn usable_name(n: &str) -> bool {
    !n.is_empty() && n != "Driver" && !n.eq_ignore_ascii_case("bbs") && !n.eq_ignore_ascii_case("Bus Company Simulator")
}

/// After the situation is applied (the trip's map is known).
pub(crate) fn apply(args: &mut Args) {
    if !started_by_bcs(args) || args.lan_join.is_some() || args.lan_host.is_some() || args.server.is_some() || args.offscreen.is_some() {
        return;
    }
    // BCS's personnel file is named "Bus Company Simulator" for every player: the name is
    // the driver profile chosen in openOMSI's launcher, else the Windows account's
    let given = args.lan_name.trim();
    if given.is_empty() || given == "Driver" {
        let profile = omsi_launcher_lib::load_config().profile.trim().to_string();
        let name = Some(profile)
            .filter(|n| usable_name(n))
            .or_else(|| std::env::var("USERNAME").ok().map(|u| u.trim().to_string()).filter(|u| usable_name(u)));
        if let Some(n) = name {
            args.lan_name = n;
        }
    }
    let dir = directory();
    let url = format!("http://{dir}/stalium/bcs?carte={}&nom={}", encode(&args.map), encode(args.lan_name.trim()));
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(4)).timeout(Duration::from_secs(6)).build();
    let answer = match agent.get(&url).call().ok().and_then(|r| r.into_string().ok()) {
        Some(a) => a,
        None => {
            log::info!("Stalium: BCS trip on {}: no answer from the Stalium directory, driven alone", args.map);
            return;
        }
    };
    match target(&answer, &dir) {
        Some(t) => {
            log::info!("Stalium: BCS trip on {} as '{}': joining the Stalium server", args.map, args.lan_name);
            args.lan_join = Some(t);
        }
        None => log::info!("Stalium: BCS trip on {} as '{}': {}", args.map, args.lan_name, answer.trim().trim_start_matches("solo").trim()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers() {
        assert!(directory().contains(':'));
        assert_eq!(target("join 27017\n", "1.2.3.4:27027").as_deref(), Some("1.2.3.4:27017"));
        assert_eq!(target("join 5.6.7.8:27015", "1.2.3.4:27027").as_deref(), Some("5.6.7.8:27015"));
        assert_eq!(target("solo pas membre", "1.2.3.4:27027"), None);
        assert_eq!(target("join ", "1.2.3.4:27027"), None);
        assert_eq!(encode("maps/St-Servan/global.cfg"), "maps%2FSt-Servan%2Fglobal.cfg");
        assert_eq!(encode("Jean é"), "Jean%20%C3%A9");
    }
}
