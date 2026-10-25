//! Stalium: the BCS directory (`bcs.txt` beside `server.cfg`, scrambled).
//!
//! The Stalium game, started by BCS for a trip, asks this server where to go:
//! `GET /stalium/bcs?carte=maps/st-servan/global.cfg&nom=<player>` on its web port. The answer
//! is `join <port>` (the server of that map, on this same machine), `join <host:port>` or
//! `solo <why>`.
//!
//! `bcs.txt` is kept sealed with the Windows key of the server's machine (DPAPI, machine
//! scope): unreadable as it lies on the disk, and on any other computer - no key is in the
//! program or its source (elsewhere than on Windows it is only scrambled). It is edited with
//! `openomsi.exe --stalium-bcs <path of bcs.txt>`: the game opens it, readable, in Notepad and
//! scrambles it again once Notepad is closed. Readable, it is one map a line (`#` starts a
//! comment), read again on every question:
//!
//! ```text
//! maps/st-servan/global.cfg      = 27017
//! maps/berlin-spandau/global.cfg = 27021
//! maps/autre/global.cfg          = 1.2.3.4:27015
//! ```
//!
//! No `bcs.txt` (or one that does not open here): the directory is off (404).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The file's name, beside `server.cfg`.
pub const FILE_NAME: &str = "bcs.txt";

static FILE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// The file sealed with the machine's Windows key (DPAPI): readable on this server only.
const MARK_SEALED: &str = "STALIUM-BCS2:";
/// The file only scrambled (a server that is not on Windows).
const MARK: &str = "STALIUM-BCS:";
const KEY: &[u8] = b"Stalium/bcs.txt#";

fn xor(data: &[u8]) -> Vec<u8> {
    data.iter().enumerate().map(|(i, b)| b ^ KEY[i % KEY.len()]).collect()
}

fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(hex: &str) -> Option<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return None;
    }
    (0..hex.len() / 2).map(|i| u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok()).collect()
}

/// Seal (`protect`) or open `data` with this machine's key: any account of this Windows
/// (the server's service account as the administrator who edits the file) opens it, no
/// other computer does - the program and its source hold no key.
#[cfg(windows)]
fn dpapi(data: &[u8], protect: bool) -> Option<Vec<u8>> {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPTPROTECT_LOCAL_MACHINE, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB};
    let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
    let mut out = CRYPT_INTEGER_BLOB::default();
    let flags = CRYPTPROTECT_UI_FORBIDDEN | CRYPTPROTECT_LOCAL_MACHINE;
    // SAFETY: `input` points at `data` for the call; `out` is the system's buffer, copied
    // and then freed with LocalFree as the documentation asks
    unsafe {
        let r = if protect {
            CryptProtectData(&input, windows::core::w!("Stalium bcs.txt"), None, None, None, flags, &mut out)
        } else {
            CryptUnprotectData(&input, None, None, None, None, flags, &mut out)
        };
        r.ok()?;
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(out.pbData as _)));
        Some(v)
    }
}

/// The file as it is written on the disk: sealed with this machine's key on Windows,
/// scrambled elsewhere.
pub fn scramble(plain: &str) -> String {
    #[cfg(windows)]
    if let Some(sealed) = dpapi(plain.as_bytes(), true) {
        return format!("{MARK_SEALED}{}\r\n", hex(&sealed));
    }
    format!("{MARK}{}\r\n", hex(&xor(plain.as_bytes())))
}

/// The file readable again (None: not a bcs.txt of this server).
pub fn unscramble(file: &str) -> Option<String> {
    let file = file.trim().trim_start_matches('\u{feff}');
    if let Some(h) = file.strip_prefix(MARK_SEALED) {
        #[cfg(windows)]
        return dpapi(&unhex(h.trim())?, false).map(|p| String::from_utf8_lossy(&p).into_owned());
        #[cfg(not(windows))]
        return {
            let _ = h;
            None
        };
    }
    let bytes = unhex(file.strip_prefix(MARK)?.trim())?;
    Some(String::from_utf8_lossy(&xor(&bytes)).into_owned())
}

const TEMPLATE: &str = "# Trajets BCS : une carte par ligne, carte = port (sur ce serveur) ou adresse:port\r\n# Enregistre puis ferme le Bloc-notes : le fichier est brouille a nouveau.\r\nmaps/st-servan/global.cfg = 27017\r\n";

/// `openomsi.exe --stalium-bcs <bcs.txt>`: the file opened readable in Notepad (the editor
/// of $EDITOR elsewhere), scrambled again once it is closed. A missing file starts from an
/// example.
pub fn edit(path: &Path) -> Result<(), String> {
    let plain = match std::fs::read_to_string(path) {
        Ok(t) => unscramble(&t).ok_or_else(|| format!("{} ne s'ouvre pas : ce n'est pas un bcs.txt, ou il a ete chiffre sur un autre serveur (recree-le ici)", path.display()))?,
        Err(_) => TEMPLATE.to_string(),
    };
    let tmp = std::env::temp_dir().join(format!("bcs-{}.txt", std::process::id()));
    std::fs::write(&tmp, &plain).map_err(|e| format!("{}: {e}", tmp.display()))?;
    let editor = if cfg!(windows) { "notepad.exe".to_string() } else { std::env::var("EDITOR").unwrap_or_else(|_| "nano".into()) };
    let ran = std::process::Command::new(&editor).arg(&tmp).status();
    let edited = std::fs::read_to_string(&tmp);
    let _ = std::fs::remove_file(&tmp);
    ran.map_err(|e| format!("{editor}: {e}"))?;
    let edited = edited.map_err(|e| e.to_string())?;
    std::fs::write(path, scramble(&edited)).map_err(|e| format!("{}: {e}", path.display()))?;
    let sealed = std::fs::read_to_string(path).map(|t| t.starts_with(MARK_SEALED)).unwrap_or(false);
    println!("{} : {} carte(s), fichier {}", path.display(), parse(&edited).len(), if sealed { "chiffre avec la cle de ce serveur" } else { "brouille" });
    Ok(())
}

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

/// The answer to a question.
pub fn route(routes: &[(String, String)], map: &str) -> String {
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
    let Some(text) = std::fs::read_to_string(&file).ok().and_then(|t| unscramble(&t)) else {
        log::warn!("server: BCS : {} manque ou ne s'ouvre pas sur ce serveur (openomsi.exe --stalium-bcs pour le creer)", file.display());
        return None;
    };
    let map = query_value(query, "carte");
    let name = query_value(query, "nom");
    let a = route(&parse(&text), &map);
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
        assert_eq!(route(&r, "maps/st-servan/global.cfg"), "join 27017");
        assert_eq!(route(&r, "Maps/Berlin-Spandau/global.cfg"), "join 1.2.3.4:27021");
        assert!(route(&r, "maps/grundorf/global.cfg").starts_with("solo"));
        let f = scramble("maps/a/global.cfg = 27017\r\n");
        assert!((f.starts_with(MARK) || f.starts_with(MARK_SEALED)) && !f.contains("27017") && !f.contains("maps"));
        assert_eq!(unscramble(&f).as_deref(), Some("maps/a/global.cfg = 27017\r\n"));
        assert_eq!(unscramble("maps/a/global.cfg = 1"), None);
        assert_eq!(unscramble(TEMPLATE).map(|_| ()), None);
        assert_eq!(query_value("carte=maps%2FSt-Servan%2Fglobal.cfg&nom=Jean+Dupont", "nom"), "Jean Dupont");
        assert_eq!(query_value("carte=maps%2FSt-Servan%2Fglobal.cfg&nom=x", "carte"), "maps/St-Servan/global.cfg");
        assert_eq!(decode("a%"), "a%");
    }
}
