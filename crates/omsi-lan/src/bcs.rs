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
//! cle: <the company's BCS API key>
//! ```
//!
//! With one `cle:` line or more (a company each), only the staff of those companies joins:
//! the player's name (its openOMSI profile) must be the `username` of one of their
//! employees in BCS (pedepe.de's `staffjson`, asked again every 5 minutes at most). Without
//! any, every player of the Stalium game joins.
//!
//! No `bcs.txt` (or one that does not open here): the directory is off (404).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

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

const TEMPLATE: &str = "# ================================================================\r\n#  bcs.txt - trajets BCS (Bus Company Simulator) vers vos serveurs\r\n# ================================================================\r\n#\r\n# Quand BCS lance un trajet avec la version Stalium d'openOMSI, le jeu\r\n# demande a ce serveur ou aller. Ce fichier donne la reponse.\r\n# Les lignes qui commencent par # sont des explications : elles ne\r\n# comptent pas. Retirez le # devant une ligne d'exemple pour l'utiliser.\r\n#\r\n# 1) LES CARTES : une ligne par carte, sous la forme\r\n#       carte = port\r\n#    ou carte = adresse:port\r\n#    - carte : le dossier de la carte dans OMSI, en minuscules,\r\n#      sous la forme maps/<dossier>/global.cfg\r\n#    - port seul : le serveur openOMSI de cette carte tourne sur la\r\n#      meme machine que celui-ci (son port de jeu, pas son port web)\r\n#    - adresse:port : le serveur de cette carte est sur une autre machine\r\n#    Une carte absente de la liste se joue en solo.\r\n#\r\n# maps/ma-carte/global.cfg = 27015\r\n# maps/autre-carte/global.cfg = adresse-du-serveur:27016\r\n#\r\n# 2) LES ENTREPRISES (facultatif) : une ligne par entreprise BCS\r\n#       cle: <cle API de l'entreprise>\r\n#    La cle se trouve dans BCS : Compagnie > API. Avec au moins une\r\n#    ligne cle:, seuls les employes de ces entreprises rejoignent le\r\n#    serveur ; les autres roulent en solo. Sans ligne cle:, tous les\r\n#    joueurs de cette version rejoignent.\r\n#    Chaque joueur doit avoir dans le lanceur openOMSI un profil\r\n#    conducteur au nom exact de son pseudo BCS.\r\n#\r\n# cle: XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX\r\n#\r\n# Enregistrez puis fermez le Bloc-notes : le fichier est chiffre a\r\n# nouveau avec la cle Windows de ce serveur (illisible ailleurs).\r\n# Pour le modifier : openomsi.exe --stalium-bcs bcs.txt\r\n";

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
        .filter(|l| !l.is_empty() && !l.starts_with('#') && company_key(l).is_none())
        .filter_map(|l| l.split_once('='))
        .map(|(m, t)| (canonical_map(m), t.trim().to_string()))
        .filter(|(m, t)| !m.is_empty() && !t.is_empty())
        .collect()
}

/// A `cle: …` (or `key: …`) line's API key.
fn company_key(line: &str) -> Option<String> {
    let (k, v) = line.split_once(':')?;
    matches!(k.trim().to_lowercase().as_str(), "cle" | "clé" | "key").then(|| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// The companies' API keys of a `bcs.txt`.
pub fn keys(text: &str) -> Vec<String> {
    text.lines().map(|l| l.trim().trim_start_matches('\u{feff}')).filter(|l| !l.starts_with('#')).filter_map(company_key).collect()
}

/// The players among a `staffjson` answer's employees (BCS's own AI drivers left out), as
/// names compare: lower case, no spaces at the ends.
pub fn staff_names(json: &str) -> Option<Vec<String>> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let list = v.get("employees")?.as_array()?;
    Some(
        list.iter()
            .filter_map(|e| e.get("username")?.as_str())
            .map(|n| n.trim().to_lowercase())
            .filter(|n| !n.is_empty() && !n.starts_with("(ai)"))
            .collect(),
    )
}

const STAFF_AGE: Duration = Duration::from_secs(300);
/// Per key: when its staff was last asked for, and the names (kept when pedepe.de does not
/// answer later).
static STAFF: Mutex<Option<HashMap<String, (Instant, Vec<String>)>>> = Mutex::new(None);

fn staff_of(key: &str) -> Vec<String> {
    {
        let g = STAFF.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, names)) = g.as_ref().and_then(|m| m.get(key)) {
            if at.elapsed() < STAFF_AGE {
                return names.clone();
            }
        }
    }
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(4)).timeout(Duration::from_secs(8)).build();
    let fresh = agent
        .get("https://pedepe.de/api/api.php")
        .query("key", key)
        .query("request", "staffjson")
        .call()
        .ok()
        .and_then(|r| r.into_string().ok())
        .and_then(|t| staff_names(&t));
    let mut g = STAFF.lock().unwrap_or_else(|e| e.into_inner());
    let m = g.get_or_insert_with(HashMap::new);
    match fresh {
        Some(names) => {
            m.insert(key.to_string(), (Instant::now(), names.clone()));
            names
        }
        None => {
            log::warn!("server: BCS : pedepe.de ne donne pas le personnel d'une entreprise (cle fausse ou site injoignable)");
            m.get(key).map(|(_, n)| n.clone()).unwrap_or_default()
        }
    }
}

/// Whether `name` may join: no key, everybody; else an employee of one of the companies.
fn employed(keys: &[String], name: &str) -> bool {
    let n = name.trim().to_lowercase();
    keys.is_empty() || (!n.is_empty() && keys.iter().any(|k| staff_of(k).contains(&n)))
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
    let a = if employed(&keys(&text), &name) { route(&parse(&text), &map) } else { "solo pas employe de l'entreprise".into() };
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
        let t = "maps/a/global.cfg = 1\r\ncle: ABC-1\r\n# cle: off\r\nKey : XYZ\r\n";
        assert_eq!(keys(t), ["ABC-1", "XYZ"]);
        assert_eq!(parse(t).len(), 1);
        assert!(employed(&[], "n'importe qui"));
        let staff = r#"{"employees":[{"employeeId":1,"username":"Michael75","title":"-"},{"employeeId":2,"username":"(AI) Vera S."}],"openApplications":0}"#;
        assert_eq!(staff_names(staff), Some(vec!["michael75".to_string()]));
        assert_eq!(staff_names("<html>"), None);
        let f = scramble("maps/a/global.cfg = 27017\r\n");
        assert!((f.starts_with(MARK) || f.starts_with(MARK_SEALED)) && !f.contains("27017") && !f.contains("maps"));
        assert_eq!(unscramble(&f).as_deref(), Some("maps/a/global.cfg = 27017\r\n"));
        assert_eq!(unscramble("maps/a/global.cfg = 1"), None);
        assert_eq!(unscramble(TEMPLATE).map(|_| ()), None);
        assert!(parse(TEMPLATE).is_empty() && keys(TEMPLATE).is_empty());
        assert_eq!(query_value("carte=maps%2FSt-Servan%2Fglobal.cfg&nom=Jean+Dupont", "nom"), "Jean Dupont");
        assert_eq!(query_value("carte=maps%2FSt-Servan%2Fglobal.cfg&nom=x", "carte"), "maps/St-Servan/global.cfg");
        assert_eq!(decode("a%"), "a%");
    }
}
