//! Stalium Transport: a BCS (Busbetrieb-Simulator) trip on a Stalium map joins the
//! Stalium server by itself, with nothing to set in BCS (it only picks openOMSI's folder).
//!
//! BCS starts the game with its own personnel file (`--driver …\\.openomsi\\bbs\\bbs.odr`),
//! `--no-menu` and the trip's situation. On such a start, with no LAN option of its own,
//! the map of the trip is looked up in the list below: a Stalium map gets
//! `--lan-join <server>` and the player's openOMSI profile name; any other map is
//! driven alone, as before. The list is kept scrambled so that it is no file for the
//! players to read (it is no secret either: the server's address is public).

use crate::cli::Args;

const KEY: &[u8] = b"Stalium-BCS-2026";
/// `map=host:port`, one a line, XOR [`KEY`] (written by scripts/stalium/servers.py).
const SERVERS: &[u8] = &[
    0x3e, 0x15, 0x11, 0x1f, 0x46, 0x17, 0x08, 0x5f, 0x2e, 0x2a, 0x3d, 0x00, 0x41, 0x40, 0x53, 0x58,
    0x37, 0x15, 0x14, 0x43, 0x0e, 0x19, 0x02, 0x4f, 0x23, 0x2f, 0x7d, 0x4e, 0x54, 0x57, 0x0f, 0x0e,
    0x61, 0x5a, 0x50, 0x5a, 0x5c, 0x5b, 0x5b, 0x1a, 0x6c, 0x72, 0x61, 0x1a, 0x08, 0x02, 0x05, 0x06,
    0x61, 0x45, 0x6b, 0x01, 0x08, 0x05, 0x1e, 0x02, 0x31, 0x37, 0x7e, 0x5e, 0x57, 0x42, 0x44, 0x57,
    0x3d, 0x5b, 0x06, 0x00, 0x06, 0x17, 0x0c, 0x41, 0x6c, 0x20, 0x35, 0x4a, 0x0f, 0x08, 0x00, 0x18,
    0x62, 0x42, 0x54, 0x42, 0x5f, 0x42, 0x43, 0x1c, 0x70, 0x74, 0x69, 0x1f, 0x05, 0x00, 0x03, 0x01
];

fn servers() -> Vec<(String, String)> {
    let plain: Vec<u8> = SERVERS.iter().enumerate().map(|(i, b)| b ^ KEY[i % KEY.len()]).collect();
    String::from_utf8_lossy(&plain)
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(m, a)| (m.trim().to_string(), a.trim().to_string()))
        .collect()
}

/// `maps/St-Servan/global.cfg`, `.\Maps\st-servan\global.cfg` … as one name.
fn canonical_map(map: &str) -> String {
    let m = map.replace('\\', "/").to_lowercase();
    let m = m.trim_start_matches("./");
    match m.find("maps/") {
        Some(i) => m[i..].to_string(),
        None => m.to_string(),
    }
}

/// A start by BCS: its own personnel file `bbs\bbs.odr`.
fn started_by_bcs(args: &Args) -> bool {
    args.driver.as_deref().map(|d| d.replace('\\', "/").to_lowercase().ends_with("bbs/bbs.odr")).unwrap_or(false)
}

fn server_for(map: &str) -> Option<String> {
    let m = canonical_map(map);
    servers().into_iter().find(|(k, _)| *k == m).map(|(_, a)| a)
}

/// After the situation is applied (the trip's map is known).
pub(crate) fn apply(args: &mut Args) {
    if !started_by_bcs(args) || args.lan_join.is_some() || args.lan_host.is_some() || args.server.is_some() || args.offscreen.is_some() {
        return;
    }
    let Some(addr) = server_for(&args.map) else {
        log::info!("Stalium: BCS trip on {}, no Stalium server for this map: driven alone", args.map);
        return;
    };
    log::info!("Stalium: BCS trip on {}: joining the Stalium server", args.map);
    args.lan_join = Some(addr);
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
}

fn usable_name(n: &str) -> bool {
    !n.is_empty() && n != "Driver" && !n.eq_ignore_ascii_case("bbs") && !n.eq_ignore_ascii_case("Bus Company Simulator")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_and_maps() {
        assert_eq!(servers().len(), 2);
        assert!(server_for("maps/St-Servan/global.cfg").is_some());
        assert!(server_for(r"Maps\Berlin-Spandau\global.cfg").is_some());
        assert!(server_for("maps/Grundorf/global.cfg").is_none());
        assert!(!servers().iter().any(|(m, a)| m.contains(' ') || a.is_empty()));
    }
}
