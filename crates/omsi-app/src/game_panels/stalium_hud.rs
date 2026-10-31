//! Stalium Transport's HUD, part of the game (it was a Lua plugin, `fr.stalium.route`, to be
//! copied into the game's folder): a bar along the top of the screen, the driver's tablet and
//! the road sheet.
//!
//! - The bar (always, while the setting `stalium_hud` is on and the player drives), a block
//!   at the top in two lines: the line and its terminus, the time, the speed (red over the
//!   road's limit) and the limit, the next stop and how far it is, the delay in words; under
//!   them the driver, the bus, the sheet so far, the passengers and the tank with their gauges.
//!   It takes the place of the game's information bar.
//! - The tablet (`stalium_tablet`, F11 by default): the line and the duty, the next stop and
//!   the delay, the terminus, the passengers and the takings, and the road sheet of the trip
//!   (when it began, the distance, the passengers carried, the tickets) - sent by itself as
//!   the bus reaches the terminus (on a Stalium server, with the player's account linked) to
//!   the Stalium website ([`SITE_API`]: the plugin's `SUBMIT_DRIVER_SHEET` JSON, `saved_at`
//!   and the player's key; the site answers `success` and `earned`) and to the Discord bot on
//!   this computer (UDP 127.0.0.1:47800), after which a new sheet begins. Moved by dragging
//!   its free parts, resized by dragging its corner; both kept.
//! - `stalium_hud` (F7): the HUD hidden or shown again.
//!
//! The tablet and the sheet take the mouse while open (Esc closes them); the bar does not.

use super::*;
use omsi_plugin::ui::Anchor;

/// The Discord bot's / Stalium Live's port on this computer.
pub(crate) const BOT_PORT: u16 = 47800;
/// Where the road sheets go on the Stalium website (Stalium Companion's `api_url`).
pub(crate) const SITE_API: &str = "https://omsi.stalium-transport.fr/api/omsi_trips.php";

const GREY: Rgba = Rgba([0x7F, 0x8C, 0x8D, 255]);
const LIGHT: Rgba = Rgba([0xBD, 0xC3, 0xC7, 255]);
const BLUE: Rgba = Rgba([0x3B, 0x8B, 0xEB, 255]);
const GREEN: Rgba = Rgba([0x2E, 0xCC, 0x71, 255]);
const RED: Rgba = Rgba([0xE7, 0x4C, 0x3C, 255]);
const WHITE: Rgba = Rgba([255, 255, 255, 255]);
/// The logo's blue (its S when the picture cannot be drawn).
const LOGO_BLUE: Rgba = Rgba([0x1E, 0x88, 0xF5, 255]);
/// The line's badge and the speed limit's sign.
const LINE_RED: Rgba = Rgba([0xD9, 0x34, 0x2B, 255]);

/// The HUD's state in the game's panels.
#[derive(Default)]
pub(crate) struct StaliumHud {
    pub(crate) tablet: bool,
    /// Hidden with `stalium_hud` (F7): nothing shows, the game's information bar neither.
    pub(crate) hidden: bool,
    /// The sheet's beginning: the game time (s), the session's metres, its tickets (count,
    /// takings) and passengers then. None until the first frame with a bus.
    start: Option<SheetStart>,
    /// The last frame's values: the sheet of a trip that has just ended is that.
    last: Option<Snapshot>,
    /// The trip whose sheet has gone (or was dealt with) at its terminus: not again when the
    /// duty moves on to the next trip, a minute before it leaves.
    sent: Option<TripId>,
    /// Where the tablet stands and how big it is (dragged, made bigger or smaller by the
    /// player, kept in `~/.openomsi/stalium/tablette.txt`); read the first time it opens.
    place: Option<TabletPlace>,
    /// The tablet held by the left button: moved (by its free parts) or resized (by its
    /// corner), and where the cursor was (physical pixels).
    hold: Option<(Hold, (f32, f32))>,
}

/// The tablet's top left corner (logical pixels, None: at the right of the screen, under the
/// bar - away from the chat on the left) and its size (1 = as made).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TabletPlace {
    pub at: Option<(f32, f32)>,
    pub k: f32,
}

impl Default for TabletPlace {
    fn default() -> Self {
        TabletPlace { at: None, k: 1.0 }
    }
}

/// The tablet's smallest and largest size.
pub(crate) const TABLET_SIZES: (f32, f32) = (0.7, 1.6);
/// The tablet's width at size 1 (logical pixels).
pub(crate) const TABLET_WIDTH: f32 = 380.0;

/// What the left button does with the tablet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Hold {
    Move,
    Resize,
}

impl TabletPlace {
    /// As kept in its file: `x y k`, or `- - k` where it was never moved.
    pub(crate) fn read(text: &str) -> TabletPlace {
        let v: Vec<&str> = text.split_whitespace().collect();
        let num = |i: usize| v.get(i).and_then(|t| t.parse::<f32>().ok()).filter(|x| x.is_finite());
        let at = num(0).zip(num(1));
        let k = num(2).unwrap_or(1.0).clamp(TABLET_SIZES.0, TABLET_SIZES.1);
        TabletPlace { at, k }
    }

    pub(crate) fn text(&self) -> String {
        match self.at {
            Some((x, y)) => format!("{x:.0} {y:.0} {:.1}", self.k),
            None => format!("- - {:.1}", self.k),
        }
    }

    /// Its corner on a screen `screen` logical pixels wide.
    pub(crate) fn corner(&self, screen: f32) -> (f32, f32) {
        self.at.unwrap_or(((screen - TABLET_WIDTH * self.k - 16.0).max(0.0), 110.0))
    }
}

fn place_file() -> Option<std::path::PathBuf> {
    key_file().map(|f| f.with_file_name("tablette.txt"))
}

fn load_place() -> TabletPlace {
    place_file().and_then(|f| std::fs::read_to_string(f).ok()).map(|t| TabletPlace::read(&t)).unwrap_or_default()
}

fn save_place(p: &TabletPlace) {
    if let Some(f) = place_file() {
        let _ = f.parent().map(std::fs::create_dir_all);
        let _ = std::fs::write(f, p.text());
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct SheetStart {
    time: f64,
    metres: f64,
    tickets: (i32, f64),
    passengers: i32,
}

/// What the HUD shows, read from the game once a frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Snapshot {
    /// The keys of the tablet, the road sheet and the HUD as bound ("F11", "(unbound)").
    pub keys: [String; 2],
    /// Playing on a Stalium server (see `stalium_server_name`): only then do sheets go.
    pub stalium_server: bool,
    /// On a Stalium server, the player's multiplayer name; else the profile's (shown only).
    pub driver: String,
    pub map: String,
    pub bus: String,
    pub number: String,
    /// The game time (s since midnight).
    pub time: f64,
    pub speed_kmh: f32,
    /// The road's speed limit (km/h), as the navigator knows it.
    pub speed_limit: Option<f32>,
    /// Km of the sheet so far.
    pub km: f64,
    pub odometer_km: Option<f64>,
    pub temperatures: (f32, f32),
    /// The tank, 0..1.
    pub fuel: Option<f32>,
    pub aboard: usize,
    pub capacity: Option<usize>,
    pub line: Option<String>,
    pub tour: Option<String>,
    /// The duty's trip driven (its index), with a duty.
    pub trip: Option<usize>,
    /// The trip's last stop reached (the game's own "trip done"): only such a trip's sheet goes.
    pub trip_done: bool,
    pub terminus: Option<String>,
    pub next_stop: Option<String>,
    /// Metres to the next stop: by the road (the navigator's), else in a straight line.
    pub next_stop_m: Option<f64>,
    /// Seconds late (negative: early), with a duty.
    pub delay: Option<f64>,
    /// Tickets sold and takings, passengers carried, since the sheet began.
    pub tickets: (i32, f64),
    pub carried: i32,
    /// When the sheet began (game time, s).
    pub start: f64,
}

pub(crate) fn clock(t: f64) -> String {
    let t = t.rem_euclid(86400.0);
    format!("{:02}:{:02}:{:02}", (t / 3600.0) as i64, ((t % 3600.0) / 60.0) as i64, (t % 60.0) as i64)
}

/// `+1:05` late, `-0:40` early, and the colour of it: green within a minute either way, red
/// later, blue earlier.
pub(crate) fn delay_text(delay: f64) -> (String, Rgba) {
    let s = delay.abs().round() as i64;
    let text = format!("{}{}:{:02}", if delay < 0.0 { "-" } else { "+" }, s / 60, s % 60);
    let color = if delay > 60.0 {
        RED
    } else if delay < -60.0 {
        BLUE
    } else {
        GREEN
    };
    (text, color)
}

/// An amount as French writes it: `7,20`.
pub(crate) fn euros(x: f64) -> String {
    format!("{x:.2}").replace('.', ",")
}

/// A name cut to `max` characters, with ".." when it was longer.
pub(crate) fn short(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max.saturating_sub(2)).collect();
    format!("{}..", cut.trim_end())
}

/// The road sheet as Stalium Companion and the bot take it (the plugin's
/// `SUBMIT_DRIVER_SHEET`, the same keys).
pub(crate) fn sheet_json(s: &Snapshot) -> String {
    let esc = |t: &str| t.replace('\\', "\\\\").replace('"', "\\\"").chars().filter(|c| !c.is_control()).collect::<String>();
    let delay = s.delay.map(|d| delay_text(d).0).unwrap_or_else(|| "+0:00".into());
    format!(
        "{{\"action\":\"SUBMIT_DRIVER_SHEET\",\"driver\":\"{}\",\"map\":\"{}\",\"line\":\"{}\",\"tour\":\"{}\",\"bus\":\"{}\",\"start_time\":\"{}\",\"end_time\":\"{}\",\"distance_km\":{:.1},\"passengers\":{},\"delay\":\"{}\",\"tickets_sold\":{},\"ticket_revenue\":{:.2},\"fines_revenue\":0.00,\"frauds\":0}}",
        esc(&s.driver),
        esc(&s.map),
        esc(s.line.as_deref().unwrap_or("HLP")),
        esc(s.tour.as_deref().unwrap_or("--")),
        esc(&s.bus),
        clock(s.start),
        clock(s.time),
        s.km,
        s.carried,
        delay,
        s.tickets.0,
        s.tickets.1,
    )
}

/// The sheet for the website: the same JSON with `saved_at` (the computer's local date and
/// time, `YYYY-MM-DD HH:MM:SS`), as Stalium Companion sent it.
pub(crate) fn with_saved_at(json: &str, saved_at: &str) -> String {
    let body = json.trim_end().trim_end_matches('}');
    format!("{body},\"saved_at\":\"{saved_at}\"}}")
}

/// The sheet for the website with the player's key (`key`): the website knows the driver
/// by it, not by the name the sheet says.
pub(crate) fn with_key(json: &str, key: &str) -> String {
    let body = json.trim_end().trim_end_matches('}');
    format!("{body},\"key\":\"{key}\"}}")
}

/// A key of the website as the player copied it, tidied (`STAL-ABCD-EFGH-JKLM`); None when
/// it cannot be one (8 to 64 letters, figures and dashes).
pub(crate) fn tidy_key(text: &str) -> Option<String> {
    let k = text.trim().to_ascii_uppercase();
    (8..=64).contains(&k.len()).then_some(()).filter(|_| k.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')).map(|_| k)
}

/// Where the player's key is kept on this computer (`~/.openomsi/stalium/cle.txt`).
fn key_file() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(std::path::PathBuf::from(home).join(".openomsi").join("stalium").join("cle.txt"))
}

static KEY: std::sync::Mutex<Option<Option<String>>> = std::sync::Mutex::new(None);

/// The player's key of the Stalium website (their account linked), read once.
pub(crate) fn stalium_key() -> Option<String> {
    let mut k = KEY.lock().unwrap_or_else(|e| e.into_inner());
    k.get_or_insert_with(|| key_file().and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| tidy_key(&t))).clone()
}

/// The key kept (Some) or forgotten (None); false when it could not be written.
fn set_key(key: Option<&str>) -> bool {
    let Some(f) = key_file() else { return false };
    let done = match key {
        Some(k) => f.parent().is_some_and(|d| std::fs::create_dir_all(d).is_ok()) && std::fs::write(&f, k).is_ok(),
        None => !f.exists() || std::fs::remove_file(&f).is_ok(),
    };
    if done {
        *KEY.lock().unwrap_or_else(|e| e.into_inner()) = Some(key.map(str::to_string));
    }
    done
}

/// The text on the clipboard, where there is one to read.
fn from_clipboard() -> Option<String> {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        arboard::Clipboard::new().ok()?.get_text().ok()
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        None
    }
}

fn saved_at_now() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() % 60).unwrap_or(0);
    match omsi_launcher_lib::local_now() {
        Some((y, mo, d, h, mi)) => format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{secs:02}"),
        None => String::new(),
    }
}

/// What the website said of a sheet: validated with what it earned, or why not.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SiteAnswer {
    Validated(String),
    Refused(String),
    Unreachable(String),
}

/// The website's answer read as Stalium Companion read it: `success` true and `earned`.
pub(crate) fn read_answer(body: &str) -> SiteAnswer {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
        return SiteAnswer::Refused(short(body, 80));
    };
    let ok = match &v["success"] {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        serde_json::Value::String(t) => matches!(t.trim().to_ascii_lowercase().as_str(), "true" | "1"),
        _ => false,
    };
    if ok {
        let earned = match &v["earned"] {
            serde_json::Value::Null => "0".to_string(),
            serde_json::Value::String(t) => t.clone(),
            other => other.to_string(),
        };
        SiteAnswer::Validated(earned)
    } else {
        let why = v["message"].as_str().or(v["error"].as_str()).map(str::to_string).unwrap_or_else(|| short(body, 80));
        SiteAnswer::Refused(why)
    }
}

/// The website's answers that came, for the HUD to say (taken once a frame).
static ANSWERS: std::sync::Mutex<Vec<SiteAnswer>> = std::sync::Mutex::new(Vec::new());

/// Send the sheet to the website on a thread of its own (the game goes on; its answer is said
/// when it comes) and to the bot on this computer (UDP: nothing waits, nothing is said when it
/// does not run).
fn send_sheet(json: &str, key: &str) {
    if let Ok(socket) = std::net::UdpSocket::bind(("127.0.0.1", 0)) {
        let _ = socket.send_to(json.as_bytes(), ("127.0.0.1", BOT_PORT));
    }
    let body = with_key(&with_saved_at(json, &saved_at_now()), key);
    let spawned = std::thread::Builder::new().name("stalium-sheet".into()).spawn(move || {
        let agent = ureq::AgentBuilder::new().timeout_connect(std::time::Duration::from_secs(6)).timeout(std::time::Duration::from_secs(12)).build();
        let answer = match agent.post(SITE_API).set("Content-Type", "application/json").set("User-Agent", "openOMSI-Stalium").send_string(&body) {
            Ok(r) => read_answer(&r.into_string().unwrap_or_default()),
            // (the site's own refusal, with its words)
            Err(ureq::Error::Status(code, r)) => {
                // (its message when it gave one: "Compte non relié...")
                let text = r.into_string().unwrap_or_default();
                match read_answer(&text) {
                    SiteAnswer::Refused(why) if serde_json::from_str::<serde_json::Value>(&text).is_ok_and(|v| v["message"].is_string()) => SiteAnswer::Refused(why),
                    _ => SiteAnswer::Refused(format!("HTTP {code} {}", short(&text, 60))),
                }
            }
            Err(e) => SiteAnswer::Unreachable(e.to_string()),
        };
        log::info!("Stalium: road sheet to {SITE_API}: {answer:?}");
        ANSWERS.lock().unwrap_or_else(|e| e.into_inner()).push(answer);
    });
    if spawned.is_err() {
        ANSWERS.lock().unwrap_or_else(|e| e.into_inner()).push(SiteAnswer::Unreachable("no thread".into()));
    }
}

/// The website's answers since the last frame, said on the screen.
fn say_answers(app: &mut crate::App) {
    let answers: Vec<SiteAnswer> = std::mem::take(&mut *ANSWERS.lock().unwrap_or_else(|e| e.into_inner()));
    for a in answers {
        app.service_msg = Some(match a {
            SiteAnswer::Validated(earned) => (format!("Feuille de route validée sur stalium-transport.fr ! Gain : {earned} €"), 8.0),
            SiteAnswer::Refused(why) => (format!("Le site Stalium n'a pas validé la feuille de route : {why}"), 10.0),
            SiteAnswer::Unreachable(_) => ("Le site Stalium ne répond pas : feuille de route non envoyée (voir game.log)".into(), 10.0),
        });
    }
}

// --- what the game says --------------------------------------------------------------------

/// The Stalium server machine's addresses (see [`crate::stalium_bcs::directory_host`]): read
/// at once from an IP, else looked up once on a thread of its own (empty until then, and
/// in a build without the secret).
fn stalium_ips() -> Vec<std::net::IpAddr> {
    static IPS: std::sync::OnceLock<Vec<std::net::IpAddr>> = std::sync::OnceLock::new();
    static ASKED: std::sync::Once = std::sync::Once::new();
    if let Some(ips) = IPS.get() {
        return ips.clone();
    }
    let Some(host) = crate::stalium_bcs::directory_host() else {
        let _ = IPS.set(Vec::new());
        return Vec::new();
    };
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        let _ = IPS.set(vec![ip]);
        return vec![ip];
    }
    ASKED.call_once(|| {
        let _ = std::thread::Builder::new().name("stalium-dns".into()).spawn(move || {
            use std::net::ToSocketAddrs;
            let ips: Vec<std::net::IpAddr> = (host.as_str(), 0).to_socket_addrs().map(|a| a.map(|s| s.ip()).collect()).unwrap_or_default();
            log::info!("Stalium: the Stalium server is {ips:?}");
            let _ = IPS.set(ips);
        });
    });
    Vec::new()
}

/// Whether a game server at `host` runs on the Stalium server machine (`ips`).
pub(crate) fn is_stalium_host(host: std::net::SocketAddr, ips: &[std::net::IpAddr]) -> bool {
    let ip = host.ip().to_canonical();
    ips.iter().any(|i| i.to_canonical() == ip)
}

/// The player's name on the Stalium server they play on now (a client of a game server of the
/// Stalium machine); None elsewhere - alone, hosting, on another server: no sheet goes then.
fn stalium_server_name(app: &crate::App) -> Option<String> {
    let l = app.net.lan.as_ref()?;
    if l.role != omsi_net::Role::Client {
        return None;
    }
    let host = l.host?;
    is_stalium_host(host, &stalium_ips()).then(|| l.my_name.trim().to_string()).filter(|n| !n.is_empty())
}

/// A name that is a player's: not empty, not the game's "Driver", not BCS's personnel file
/// ("bbs", "Bus Company Simulator" - every BCS player's).
pub(crate) fn usable_name(n: &str) -> bool {
    let n = n.trim();
    !n.is_empty() && n != "Driver" && !n.eq_ignore_ascii_case("bbs") && !n.eq_ignore_ascii_case("Bus Company Simulator")
}

fn driver_name(app: &crate::App) -> String {
    if let Some(l) = app.net.lan.as_ref().filter(|l| usable_name(&l.my_name)) {
        return l.my_name.trim().to_string();
    }
    let usable = usable_name;
    if let Some(d) = app.session.career.driver.as_ref().filter(|d| usable(d.name.trim())) {
        return d.name.trim().to_string();
    }
    let name = app.args.lan_name.trim();
    if usable(name) {
        return name.to_string();
    }
    // the driver profile chosen in the launcher (read once)
    static PROFILE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let profile = PROFILE.get_or_init(|| omsi_launcher_lib::load_config().profile.trim().to_string());
    if usable(profile) {
        return profile.clone();
    }
    "Conducteur".into()
}

impl crate::App {
    /// The HUD's values now; None without a bus to drive.
    pub(crate) fn stalium_snapshot(&mut self) -> Option<Snapshot> {
        let p = self.player.as_ref()?;
        let career = &self.session.career;
        let now = SheetStart { time: self.clock.time, metres: career.metres, tickets: career.tickets, passengers: career.stepped_in };
        let start = *self.menus.game_panels.stalium.start.get_or_insert(now);
        let v = &p.vehicle;
        let map = self.world.as_ref().map(|w| if w.global.friendly_name.trim().is_empty() { w.global.name.clone() } else { w.global.friendly_name.clone() }).unwrap_or_default();
        let def = &v.ty.def;
        let bus = format!("{} {}", def.manufacturer.trim(), def.type_name.trim()).trim().to_string();
        let key = |action: &str| self.input.game_keys.iter().find(|b| b.action.eq_ignore_ascii_case(action) && b.scan_code != 0).map(|b| omsi_input::keys::key_name(b.scan_code as i64, b.modifier as i64)).unwrap_or_else(|| "(sans touche)".into());
        let keys = [key("stalium_tablet"), key("stalium_hud")];
        let on_server = stalium_server_name(self);
        let mut s = Snapshot {
            keys,
            stalium_server: on_server.is_some(),
            // (on a Stalium server, the name the server knows the player by)
            driver: on_server.filter(|n| usable_name(n)).unwrap_or_else(|| driver_name(self)),
            map: map.trim().to_string(),
            bus: if bus.is_empty() { def.path.file_stem().map(|f| f.to_string_lossy().to_string()).unwrap_or_default() } else { bus },
            number: v.number().trim().to_string(),
            time: self.clock.time,
            speed_kmh: v.physics.velocity_kmh().abs(),
            speed_limit: self.menus.navigator.as_ref().filter(|n| n.enabled).and_then(|n| n.speed_limit).filter(|l| l.is_finite() && *l > 0.0),
            km: ((career.metres - start.metres) / 1000.0).max(0.0),
            odometer_km: v.var("kmcounter_km").filter(|x| x.is_finite()).map(|km| km as f64 + v.var("kmcounter_m").unwrap_or(0.0) as f64 / 1000.0),
            temperatures: crate::app_events::vehicle_temperatures(p),
            fuel: v.var("tank_percent").filter(|x| x.is_finite()).map(|f| f.clamp(0.0, 1.0)),
            aboard: self.session.humans.as_ref().map_or(0, |h| h.riding()),
            capacity: crate::app_events::passenger_capacity(v),
            tickets: (now.tickets.0 - start.tickets.0, now.tickets.1 - start.tickets.1),
            carried: now.passengers - start.passengers,
            start: start.time,
            ..Default::default()
        };
        if let Some(d) = self.session.duty.as_ref() {
            if let Some(trip) = d.trips.get(d.trip_index) {
                let line = if trip.line.trim().is_empty() { d.line.trim() } else { trip.line.trim() };
                s.line = Some(line.to_string()).filter(|l| !l.is_empty());
                s.tour = Some(d.tour.trim().to_string()).filter(|t| !t.is_empty());
                s.trip = Some(d.trip_index);
                s.trip_done = d.trip_done();
                s.terminus = Some(trip.terminus.trim().to_string()).filter(|t| !t.is_empty());
                if let Some(st) = trip.stops.get(d.next_stop) {
                    s.next_stop = Some(st.name.trim().to_string()).filter(|n| !n.is_empty());
                    let by_road = self.menus.navigator.as_ref().filter(|n| n.enabled).and_then(|n| n.next_stop_distance_for(st.object_id));
                    let straight = st.position.map(|at| (at.x - v.position.x).hypot(at.y - v.position.y));
                    s.next_stop_m = by_road.or(straight);
                }
                s.delay = Some(v.host.tt_delay as f64);
            }
        }
        Some(s)
    }

    /// The tablet opened or closed (`stalium_tablet`).
    pub(crate) fn stalium_toggle_tablet(&mut self) {
        let h = &mut self.menus.game_panels.stalium;
        h.tablet = !h.tablet;
        if h.tablet {
            h.hidden = false;
        }
    }

    /// The HUD hidden or shown again (`stalium_hud`): hidden, its windows close too.
    pub(crate) fn stalium_toggle_hud(&mut self) {
        let h = &mut self.menus.game_panels.stalium;
        h.hidden = !h.hidden;
        if h.hidden {
            h.tablet = false;
        }
        self.service_msg = Some((if h.hidden { "HUD Stalium masqué" } else { "HUD Stalium affiché" }.into(), 2.5));
    }

    /// Whether the Stalium HUD takes the place of the game's information bar now.
    pub(crate) fn stalium_hud_on(&self) -> bool {
        self.settings.stalium_hud
    }

    /// The sheet of the trip that has just ended (`s`, as the last frame had it) sent, and a
    /// new one begun.
    fn stalium_send_trip_sheet(&mut self, s: &Snapshot) {
        if !s.trip_done {
            // (a trip given up before its terminus: no sheet, nothing earned - a new one begins)
            log::info!("Stalium: trip {:?} of line {:?} given up before its terminus ({:.1} km): no road sheet sent", s.trip, s.line, s.km);
            self.menus.game_panels.stalium.start = None;
            self.service_msg = Some(("Course abandonnée avant le terminus : la feuille de route n'est pas envoyée".into(), 6.0));
            return;
        }
        if !s.stalium_server {
            // (alone, hosting or on another server: the sheet stays here, a new one begins)
            log::info!("Stalium: trip {:?} of line {:?} ended off the Stalium servers: no road sheet sent", s.trip, s.line);
            self.menus.game_panels.stalium.start = None;
            self.service_msg = Some(("Course terminée : hors serveur Stalium, la feuille de route n'est pas envoyée".into(), 5.0));
            return;
        }
        let Some(key) = stalium_key() else {
            // (no account linked: the website would not know whose sheet it is)
            log::info!("Stalium: trip {:?} of line {:?} ended with no Stalium account linked: no road sheet sent", s.trip, s.line);
            self.menus.game_panels.stalium.start = None;
            self.service_msg = Some(("Feuille de route non envoyée : compte Stalium non relié. Copiez votre clé sur stalium-transport.fr, puis tablette (F11) > Coller ma clé".into(), 12.0));
            return;
        };
        let json = sheet_json(s);
        log::info!("Stalium: trip {:?} of line {:?} ended: road sheet {json}", s.trip, s.line);
        send_sheet(&json, &key);
        self.menus.game_panels.stalium.start = None;
        self.service_msg = Some((format!("Course terminée : feuille de route {} envoyée au site Stalium ({:.1} km, {} voyageur(s))...", s.line.as_deref().unwrap_or("HLP"), s.km, s.carried), 6.0));
    }
}

/// A trip of a duty: its line, its duty (tour) and its index.
pub(crate) type TripId = (Option<String>, Option<String>, usize);

pub(crate) fn trip_id(s: &Snapshot) -> Option<TripId> {
    Some((s.line.clone(), s.tour.clone(), s.trip?))
}

/// Whether `now` has just reached its trip's terminus (the game's trip done) and its sheet has
/// not gone yet (`sent`): it goes at once - not when the duty moves on to the next trip, which
/// is a minute before that one leaves (a bus 17 minutes early would wait 18).
pub(crate) fn terminus_reached(sent: Option<&TripId>, now: &Snapshot) -> bool {
    now.trip_done && trip_id(now).is_some_and(|id| sent != Some(&id))
}

/// Whether the trip of `last` (the last frame) has ended by `now`: the duty has gone on to
/// another trip, or ended, or another duty has begun - with the bus still there (a bus being
/// replaced is no trip's end) and something driven to report.
pub(crate) fn trip_ended(last: &Snapshot, now: &Snapshot) -> bool {
    last.trip.is_some() && (now.trip != last.trip || now.tour != last.tour || now.line != last.line && now.trip.is_none()) && (last.km >= 0.05 || last.carried > 0)
}

// --- the panels ----------------------------------------------------------------------------

fn colored(mut e: Element, c: Rgba) -> Element {
    e.color = Some(c);
    e
}

fn icon(name: &str, c: Rgba) -> Element {
    colored(el(None, Kind::Icon { name: name.to_string(), size: 16.0 }), c)
}

fn badge(s: &str, c: Rgba) -> Element {
    colored(el(None, Kind::Badge { text: s.to_string(), text_color: Some(WHITE), weight: Weight::Medium }), c)
}

/// `850 m`, `1,3 km`: a distance as the bar says it (tens of metres under a kilometre).
pub(crate) fn distance_text(m: f64) -> String {
    if m < 995.0 {
        format!("{:.0} m", ((m.max(0.0) / 10.0).round() * 10.0))
    } else {
        format!("{:.1} km", m / 1000.0).replace('.', ",")
    }
}

/// `7:06 d'avance`, `2:10 de retard`, `45 s d'avance` within a minute, `À l'heure` on the
/// second, and its colour.
pub(crate) fn delay_words(delay: f64) -> (String, Rgba) {
    let s = delay.abs().round() as i64;
    let mmss = format!("{}:{:02}", s / 60, s % 60);
    if delay > 60.0 {
        (format!("{mmss} de retard"), RED)
    } else if delay < -60.0 {
        (format!("{mmss} d'avance"), BLUE)
    } else if s == 0 {
        ("À l'heure".into(), GREEN)
    } else {
        // (within a minute: the seconds, green - on time)
        (format!("{s} s {}", if delay < 0.0 { "d'avance" } else { "de retard" }), GREEN)
    }
}

fn text(s: &str, size: f32, weight: Weight, c: Rgba) -> Element {
    colored(el(None, Kind::Text { text: s.to_string(), size, weight, align: Align::Left, wrap: false }), c)
}

fn gauge(value: f32, c: Rgba) -> Element {
    colored(el(None, Kind::Bar { value: value.clamp(0.0, 1.0), height: 6.0, background: Some(Rgba([0x2C, 0x31, 0x3B, 255])), segments: Vec::new() }), c)
}

/// The bar along the top (model A): the line and its terminus, the time, the speed and the
/// road's limit, the next stop and how far it is, the delay; under them the driver, the bus,
/// the sheet so far, the passengers and the tank. `width`: the screen's logical width.
pub(crate) fn bar(s: &Snapshot, width: f32) -> Panel {
    let wide = width >= 1280.0;
    let vsep = || colored(divider(), Rgba([0x2C, 0x31, 0x3B, 255]));
    let mut top = vec![match logo_path() {
        Some(path) => el(None, Kind::Image { path: Some(path), width: 24.0, height: 24.0 }),
        None => colored(el(None, Kind::Badge { text: "S".into(), text_color: Some(WHITE), weight: Weight::Bold }), LOGO_BLUE),
    }];
    match (s.line.as_deref(), s.terminus.as_deref()) {
        (Some(l), d) => {
            top.push(colored(el(None, Kind::Badge { text: short(l, 6), text_color: Some(WHITE), weight: Weight::Bold }), LINE_RED));
            if let Some(d) = d {
                top.push(text(&short(d, 22), 15.0, Weight::Medium, WHITE));
            }
        }
        (None, Some(d)) => top.push(text(&short(d, 22), 15.0, Weight::Medium, WHITE)),
        (None, None) => top.push(text("Sans service", 15.0, Weight::Medium, GREY)),
    }
    top.extend([vsep(), text(&clock(s.time)[..5], 16.0, Weight::Medium, WHITE), vsep()]);
    // the speed, red over the road's limit
    let over = s.speed_limit.is_some_and(|l| s.speed_kmh > l + 3.0);
    top.push(text(&format!("{:.0}", s.speed_kmh), 22.0, Weight::Bold, if over { RED } else { WHITE }));
    top.push(text("km/h", 12.0, Weight::Regular, LIGHT));
    if let Some(l) = s.speed_limit {
        top.push(colored(el(None, Kind::Badge { text: format!("{l:.0}"), text_color: Some(WHITE), weight: Weight::Bold }), LINE_RED));
    }
    if let Some(n) = s.next_stop.as_deref() {
        top.push(vsep());
        top.push(icon("location_on", BLUE));
        top.push(text("Prochain arrêt", 12.0, Weight::Regular, LIGHT));
        top.push(Element { grow: true, ..text(&short(n, 26), 15.0, Weight::Medium, WHITE) });
        // (at the stop, no "à 0 m")
        if let Some(m) = s.next_stop_m.filter(|m| *m >= 15.0) {
            top.push(text(&format!("à {}", distance_text(m)), 14.0, Weight::Medium, LIGHT));
        }
    }
    if let Some(d) = s.delay {
        let (words, c) = delay_words(d);
        top.push(badge(&words, c));
    }

    let bus = if s.number.is_empty() { short(&s.bus, 30) } else { format!("{} #{}", short(&s.bus, 28), s.number) };
    let mut under = vec![text(&short(&s.driver, 18), 12.0, Weight::Medium, WHITE), t_small(&bus), t_small(&format!("Ext. {:.0} °C • Bus {:.0} °C", s.temperatures.0, s.temperatures.1))];
    // (out of service: no sheet, no passengers to carry - the bar keeps to the bus)
    let in_service = s.trip.is_some();
    if in_service {
        under.push(t_small(&format!("Feuille {} • {} voyageur{}", distance_text(s.km * 1000.0), s.carried, if s.carried > 1 { "s" } else { "" })));
    }
    if wide && in_service {
        let pax = match s.capacity {
            Some(c) => format!("Passagers {}/{}", s.aboard, c),
            None => format!("Passagers {}", s.aboard),
        };
        under.push(t_small(&pax));
        if let Some(c) = s.capacity.filter(|c| *c > 0) {
            under.push(gauge(s.aboard as f32 / c as f32, BLUE));
        }
    }
    if wide {
        if let Some(f) = s.fuel {
            under.push(t_small(&format!("Carburant {:.0} %", f * 100.0)));
            under.push(gauge(f, if f < 0.15 { RED } else { GREEN }));
        }
    }

    // (as wide as what it shows, both lines from the left: the game lays a panel out at the
    // width it is given)
    let (top, under) = (el(None, Kind::Row { children: top, gap: 8.0, align: RowAlign::Start }), el(None, Kind::Row { children: under, gap: 8.0, align: RowAlign::Start }));
    let fit = (natural_width(&top).max(natural_width(&under)) + 2.0 * 10.0 + 4.0).min((width - 40.0).max(320.0));
    let mut p = card(Anchor::Top, 0.0, 12.0, fit, 235, vec![top, colored(divider(), Rgba([0x23, 0x28, 0x32, 255])), under]);
    p.padding = 10.0;
    p.gap = 7.0;
    p.radius = 14.0;
    p.background = Some(Rgba([0x0F, 0x11, 0x16, 235]));
    p
}

/// Stalium Transport's logo, in the program: written once into `~/.openomsi/stalium/` for
/// the panels to draw (they draw pictures from files); None when it cannot be (an S then).
static LOGO: &[u8] = include_bytes!("../../../../assets/stalium/logo.png");

fn logo_path() -> Option<std::path::PathBuf> {
    static PATH: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
        let dir = std::path::PathBuf::from(home).join(".openomsi").join("stalium");
        let path = dir.join("logo.png");
        if std::fs::read(&path).ok().as_deref() != Some(LOGO) {
            std::fs::create_dir_all(&dir).ok()?;
            std::fs::write(&path, LOGO).ok()?;
        }
        Some(path)
    })
    .clone()
}

/// How wide the game draws an element in a row (logical pixels), measured with its own font
/// as its panels measure (`omsi_plugin_ui`): the HUD's bar is made that wide.
pub(crate) fn natural_width(e: &Element) -> f32 {
    thread_local! {
        static FONTS: omsi_ui::Fonts = omsi_ui::Fonts::new();
    }
    fn weight(w: Weight) -> omsi_ui::Weight {
        match w {
            Weight::Regular => omsi_ui::Weight::Regular,
            Weight::Medium => omsi_ui::Weight::Medium,
            Weight::Bold => omsi_ui::Weight::Bold,
        }
    }
    FONTS.with(|f| {
        fn of(f: &omsi_ui::Fonts, e: &Element, weight: &dyn Fn(Weight) -> omsi_ui::Weight) -> f32 {
            match &e.kind {
                Kind::Text { text, size, weight: w, .. } => text.split('\n').map(|l| f.width_as_is(l, *size, weight(*w))).fold(0.0, f32::max).ceil(),
                Kind::Icon { size, .. } => *size,
                Kind::Badge { text, weight: w, .. } => (f.width_as_is(text, 12.0, weight(*w)) + 16.0).ceil(),
                Kind::Bar { .. } => 60.0,
                Kind::Divider => 1.0,
                Kind::Space { size } => *size,
                Kind::Image { width, .. } => *width,
                Kind::Row { children, gap, .. } => children.iter().map(|c| of(f, c, weight)).sum::<f32>() + gap * children.len().saturating_sub(1) as f32,
                _ => 0.0,
            }
        }
        of(f, e, &weight)
    })
}

fn t_small(s: &str) -> Element {
    text(s, 12.0, Weight::Regular, LIGHT)
}

fn row_of(children: Vec<Element>) -> Element {
    el(None, Kind::Row { children, gap: 8.0, align: RowAlign::Center })
}

/// The driver's tablet.
pub(crate) fn tablet(s: &Snapshot, at: (f32, f32), k: f32) -> Panel {
    let line = s.line.clone().unwrap_or_else(|| "DÉPÔT".into());
    let (delay, delay_color) = s.delay.map(delay_words).unwrap_or_else(|| ("--".into(), GREY));
    let pax = match s.capacity {
        Some(c) => format!("Passagers : {}/{}", s.aboard, c),
        None => format!("Passagers : {}", s.aboard),
    };
    // (its texts as big as the tablet)
    let tk = |text: &str, c: Rgba| colored(el(None, Kind::Text { text: text.to_string(), size: 13.0 * k, weight: Weight::Regular, align: Align::Left, wrap: false }), c);
    let sk = |text: &str| colored(el(None, Kind::Text { text: text.to_string(), size: 11.0 * k, weight: Weight::Regular, align: Align::Left, wrap: true }), Rgba([0x9C, 0xA3, 0xAF, 255]));
    let pair = |a: String, ca: Rgba, b: String, cb: Rgba| row_of(vec![Element { grow: true, ..tk(&a, ca) }, Element { grow: true, ..tk(&b, cb) }]);
    let service = match (s.line.as_deref(), s.tour.as_deref()) {
        (Some(l), Some(t)) => format!("LIGNE {} • SERVICE {}", short(l, 8), short(t, 6)),
        _ => format!("LIGNE {}", short(&line, 10)),
    };
    let children = vec![
        row_of(vec![
            icon("directions_bus", Rgba([0x3B, 0x82, 0xF6, 255])),
            Element { grow: true, ..colored(el(None, Kind::Text { text: "STALIUM TRANSPORT".into(), size: 14.0 * k, weight: Weight::Bold, align: Align::Left, wrap: false }), WHITE) },
            badge(&service, Rgba([0x3B, 0x82, 0xF6, 255])),
            button("close_tablet", "X", None),
        ]),
        divider(),
        row_of(vec![Element { grow: true, ..tk(&format!("Prochain : {}", s.next_stop.as_deref().unwrap_or("En attente")), WHITE) }, badge(&delay, delay_color)]),
        row_of(vec![Element { grow: true, ..tk(&format!("Terminus : {}", s.terminus.as_deref().unwrap_or("Sans service")), WHITE) }]),
        row_of(vec![Element { grow: true, ..tk(&pax, WHITE) }, badge(&format!("Caisse : {} €", euros(s.tickets.1)), Rgba([0x10, 0xB9, 0x81, 255]))]),
        divider(),
        // the road sheet of the trip (what F10 showed): it goes by itself at the terminus
        colored(el(None, Kind::Text { text: "FEUILLE EN COURS".into(), size: 11.0 * k, weight: Weight::Bold, align: Align::Left, wrap: false }), LIGHT),
        pair(format!("Prise de service : {}", clock(s.start)), WHITE, format!("Distance : {}", distance_text(s.km * 1000.0)), GREEN),
        pair(format!("Voyageurs : {}", s.carried), GREEN, format!("Billetterie : {} ticket(s)", s.tickets.0), BLUE),
        if s.stalium_server {
            sk("Serveur Stalium : la feuille part toute seule sur stalium-transport.fr à l'arrivée au terminus.")
        } else {
            sk("Hors serveur Stalium (solo, partie hébergée ou autre serveur) : la feuille n'est pas envoyée.")
        },
        divider(),
        match stalium_key() {
            Some(key) => row_of(vec![
                icon("check_circle", GREEN),
                Element { grow: true, ..tk(&format!("Compte Stalium relié (clé ...{})", &key[key.len().saturating_sub(4)..]), GREEN) },
                button("forget_key", "Retirer", None),
            ]),
            None => row_of(vec![
                icon("error", RED),
                Element { grow: true, ..tk("Compte Stalium non relié : les feuilles ne partent pas", RED) },
                button("paste_key", "Coller ma clé", Some("key")),
            ]),
        },
        sk("Votre clé openOMSI est sur stalium-transport.fr (Mon profil) : copiez-la, puis Coller ma clé."),
        row_of(vec![
            Element { grow: true, ..sk(&format!("{} : tablette • {} : masquer le HUD • Échap : fermer • glisser pour déplacer, le coin pour la taille", s.keys[0], s.keys[1])) },
            // (the corner: held and dragged, it makes the tablet bigger or smaller)
            Element { clickable: true, ..colored(el(Some("tablet_resize"), Kind::Icon { name: "fullscreen".into(), size: 16.0 }), LIGHT) },
        ]),
    ];
    let mut p = card(Anchor::TopLeft, at.0, at.1, TABLET_WIDTH * k, 250, children);
    p.accent = Some(Rgba([0x3B, 0x82, 0xF6, 255]));
    p
}

/// The tablet's new size when its corner is dragged `dx` logical pixels (wider: bigger).
pub(crate) fn resized(k: f32, dx: f32) -> f32 {
    (k + dx / TABLET_WIDTH).clamp(TABLET_SIZES.0, TABLET_SIZES.1)
}

/// A press on the tablet: on its corner it is resized, on its free parts moved, as the
/// cursor goes. False on its buttons (a click).
pub(crate) fn press(app: &mut crate::App, element: Option<&str>, at: (f32, f32)) -> bool {
    let hold = match element {
        Some("tablet_resize") => Hold::Resize,
        None => Hold::Move,
        Some(_) => return false,
    };
    let screen_width = app.menus.game_panels.ui.screen()[0];
    let h = &mut app.menus.game_panels.stalium;
    let mut p = h.place.unwrap_or_default();
    // (where it stands now, moved or not)
    p.at = Some(p.corner(screen_width));
    h.place = Some(p);
    h.hold = Some((hold, at));
    true
}

/// The cursor moved (physical pixels): the tablet held follows it. True when one was held.
pub(crate) fn drag(app: &mut crate::App, at: (f32, f32)) -> bool {
    let [w, hgt, scale] = app.menus.game_panels.ui.screen();
    let h = &mut app.menus.game_panels.stalium;
    let Some((hold, last)) = h.hold else { return false };
    let scale = scale.max(0.25);
    let (dx, dy) = ((at.0 - last.0) / scale, (at.1 - last.1) / scale);
    let mut p = h.place.unwrap_or_default();
    let (x, y) = p.corner(w);
    match hold {
        // (kept on the screen)
        Hold::Move => p.at = Some(((x + dx).clamp(0.0, (w - 60.0).max(0.0)), (y + dy).clamp(0.0, (hgt - 40.0).max(0.0)))),
        Hold::Resize => p.k = resized(p.k, dx),
    }
    h.place = Some(p);
    h.hold = Some((hold, at));
    true
}

/// The left button let go: the tablet's place and size kept. True when it was held.
pub(crate) fn release(app: &mut crate::App) -> bool {
    let h = &mut app.menus.game_panels.stalium;
    if h.hold.take().is_none() {
        return false;
    }
    if let Some(p) = h.place {
        save_place(&p);
    }
    true
}

/// The HUD's panels this frame: (id, the panel or None to take it away).
pub(crate) fn panels(app: &mut crate::App, screen_width: f32) -> Vec<(&'static str, Option<Panel>)> {
    say_answers(app);
    // (the trips' sheets go by themselves, the HUD shown or hidden)
    let mut snap = if app.stalium_hud_on() { app.stalium_snapshot() } else { None };
    if let Some(now) = snap.as_ref() {
        let last = app.menus.game_panels.stalium.last.take();
        // (out of service, the trip sent is forgotten: the same duty driven again sends again)
        if now.trip.is_none() && last.as_ref().is_none_or(|l| l.trip.is_none()) {
            app.menus.game_panels.stalium.sent = None;
        }
        let sent = app.menus.game_panels.stalium.sent.clone();
        if terminus_reached(sent.as_ref(), now) {
            // the terminus reached: the sheet goes now (something driven), and once
            app.menus.game_panels.stalium.sent = trip_id(now);
            if now.km >= 0.05 || now.carried > 0 {
                let ended = now.clone();
                app.stalium_send_trip_sheet(&ended);
                snap = app.stalium_snapshot();
            }
        } else if let Some(last) = last.filter(|l| trip_ended(l, now) && trip_id(l) != sent) {
            // a trip that ended without reaching its terminus: given up
            app.stalium_send_trip_sheet(&last);
            snap = app.stalium_snapshot();
        }
        app.menus.game_panels.stalium.last = snap.clone();
    }
    if app.menus.game_panels.stalium.tablet && app.menus.game_panels.stalium.place.is_none() {
        app.menus.game_panels.stalium.place = Some(load_place());
    }
    let on = app.stalium_hud_on() && !app.menus.game_panels.stalium.hidden;
    let snap = snap.filter(|_| on);
    // (the bar shown: the map's cards leave out what it says - the time, the speed, the next stop)
    if let Some(n) = app.menus.navigator.as_mut() {
        n.lean_cards = snap.is_some();
    }
    let h = &app.menus.game_panels.stalium;
    let Some(s) = snap else {
        return vec![("st_bar", None), ("st_tablet", None)];
    };
    let place = h.place.unwrap_or_default();
    vec![
        ("st_bar", Some(bar(&s, screen_width))),
        ("st_tablet", h.tablet.then(|| tablet(&s, place.corner(screen_width), place.k))),
    ]
}

/// A click on one of the HUD's panels.
pub(crate) fn click(app: &mut crate::App, panel: &str, element: &str) {
    match (panel, element) {
        ("st_tablet", "close_tablet") => app.menus.game_panels.stalium.tablet = false,
        ("st_tablet", "paste_key") => {
            app.service_msg = Some(match from_clipboard().as_deref().and_then(tidy_key) {
                Some(k) if set_key(Some(&k)) => ("Compte Stalium relié : vos feuilles de route partiront à votre nom".into(), 6.0),
                Some(_) => ("La clé n'a pas pu être enregistrée sur ce PC (voir game.log)".into(), 8.0),
                None => ("Pas de clé Stalium dans le presse-papiers : copiez-la sur stalium-transport.fr (Mon profil)".into(), 8.0),
            });
        }
        ("st_tablet", "forget_key") => {
            if set_key(None) {
                app.service_msg = Some(("Compte Stalium retiré de ce PC".into(), 4.0));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap() -> Snapshot {
        Snapshot {
            keys: ["F11".into(), "F7".into()],
            stalium_server: true,
            driver: "michael75".into(),
            map: "Saint-Servan".into(),
            bus: "Irisbus Citelis 18".into(),
            number: "1351".into(),
            time: 18.0 * 3600.0 + 40.0 * 60.0 + 5.0,
            speed_kmh: 32.4,
            speed_limit: Some(50.0),
            km: 12.34,
            odometer_km: Some(1_067_042.3),
            temperatures: (8.0, 21.0),
            fuel: Some(0.77),
            aboard: 12,
            capacity: Some(87),
            line: Some("21".into()),
            tour: Some("2D".into()),
            trip: Some(13),
            trip_done: true,
            terminus: Some("P+R Ronoe".into()),
            next_stop: Some("Thiers - Victor Hugo".into()),
            next_stop_m: Some(1290.0),
            delay: Some(75.0),
            tickets: (4, 7.2),
            carried: 31,
            start: 18.0 * 3600.0,
        }
    }

    #[test]
    fn times_delays_and_names_read_as_the_plugin_had_them() {
        assert_eq!(clock(18.0 * 3600.0 + 40.0 * 60.0 + 5.0), "18:40:05");
        assert_eq!(clock(86400.0 + 61.0), "00:01:01");
        assert_eq!(delay_text(75.0), ("+1:15".to_string(), RED));
        assert_eq!(delay_text(-90.0), ("-1:30".to_string(), BLUE));
        assert_eq!(delay_text(20.0).1, GREEN);
        assert_eq!(short("Thiers - Victor Hugo", 12), "Thiers - V..");
        assert_eq!(short("Docks", 12), "Docks");
    }

    /// The bar's estimated width holds all it shows: the game cuts no text of it short (and
    /// it is not much wider than that).
    #[test]
    fn the_bar_holds_its_texts() {
        let fonts = omsi_ui::Fonts::new();
        let wanted = |p: &Panel| {
            fn walk(es: &[Element], out: &mut Vec<String>) {
                for e in es {
                    match &e.kind {
                        Kind::Text { text, .. } => out.push(text.clone()),
                        Kind::Row { children, .. } => walk(children, out),
                        _ => {}
                    }
                }
            }
            let mut out = Vec::new();
            walk(&p.children, &mut out);
            out
        };
        let off = Snapshot { line: None, tour: None, trip: None, terminus: None, next_stop: None, next_stop_m: None, delay: None, ..snap() };
        for s in [snap(), off] {
            let p = bar(&s, 1920.0);
            let laid = omsi_plugin_ui::layout(&p, &fonts, 0.0);
            let drawn: Vec<String> = laid.items.iter().filter_map(|i| if let omsi_plugin_ui::Item::Text { text, .. } = i { Some(text.clone()) } else { None }).collect();
            for t in wanted(&p) {
                assert!(drawn.contains(&t), "{t:?} cut short in a bar {} wide: {drawn:?}", p.width);
            }
            // not much room left over: a little narrower and something is cut
            let mut q = p.clone();
            q.width -= 12.0;
            let laid = omsi_plugin_ui::layout(&q, &fonts, 0.0);
            let drawn: Vec<String> = laid.items.iter().filter_map(|i| if let omsi_plugin_ui::Item::Text { text, .. } = i { Some(text.clone()) } else { None }).collect();
            assert!(wanted(&q).iter().any(|t| !drawn.contains(t)), "the bar {} wide has room to spare", p.width);
        }
    }

    /// The website knows the driver by their key; a key is tidied as copied, nonsense refused.
    #[test]
    fn the_sheet_carries_the_players_key() {
        let j = with_key(&with_saved_at(&sheet_json(&snap()), "2026-10-29 21:40:00"), "STAL-ABCD-EFGH-JKLM");
        let v: serde_json::Value = serde_json::from_str(&j).expect("valid JSON");
        assert_eq!(v["key"], "STAL-ABCD-EFGH-JKLM");
        assert_eq!(v["saved_at"], "2026-10-29 21:40:00");
        assert_eq!(tidy_key("  stal-abcd-efgh-jklm\r\n"), Some("STAL-ABCD-EFGH-JKLM".into()));
        assert_eq!(tidy_key("bonjour à tous"), None);
        assert_eq!(tidy_key("abc"), None);
        assert_eq!(tidy_key("x\"}"), None);
    }

    /// BCS's personnel file is no name; the tablet says the delay as the bar does, in French.
    #[test]
    fn names_amounts_and_the_stop_reached() {
        assert!(!usable_name("bbs") && !usable_name(" Driver ") && !usable_name("Bus Company Simulator") && usable_name("michael75"));
        assert_eq!(euros(7.2), "7,20");
        let text = |p: &Panel| format!("{:?}", p.children);
        let t = tablet(&snap(), (0.0, 0.0), 1.0);
        assert!(text(&t).contains("1:15 de retard") && text(&t).contains("Caisse : 7,20 €"));
        assert!(!text(&bar(&Snapshot { next_stop_m: Some(4.0), ..snap() }, 1920.0)).contains("à 0 m"), "at the stop, no distance");
        assert!(!text(&bar(&Snapshot { next_stop_m: Some(4.0), ..snap() }, 1920.0)).contains(" à "));
    }

    /// The tablet's corner is a part a press finds (to resize it), its buttons too.
    #[test]
    fn the_tablets_corner_takes_the_press() {
        let fonts = omsi_ui::Fonts::new();
        let laid = omsi_plugin_ui::layout(&tablet(&snap(), (0.0, 0.0), 1.0), &fonts, 1.0);
        let parts: Vec<_> = laid.hits.iter().filter_map(|h| h.element.clone()).collect();
        assert!(parts.contains(&"tablet_resize".to_string()) && parts.contains(&"close_tablet".to_string()), "{parts:?}");
        let corner = laid.hits.iter().find(|h| h.element.as_deref() == Some("tablet_resize")).unwrap();
        assert!(corner.r.right() > laid.w - 40.0 && corner.r.bottom() > laid.h - 40.0, "at the bottom right: {:?} in {} x {}", corner.r, laid.w, laid.h);
    }

    #[test]
    fn distances_and_delays_read_in_words() {
        assert_eq!(distance_text(8.0), "10 m");
        assert_eq!(distance_text(846.0), "850 m");
        assert_eq!(distance_text(1290.0), "1,3 km");
        assert_eq!(delay_words(-426.0), ("7:06 d'avance".to_string(), BLUE));
        assert_eq!(delay_words(130.0), ("2:10 de retard".to_string(), RED));
        assert_eq!(delay_words(30.0), ("30 s de retard".to_string(), GREEN));
        assert_eq!(delay_words(-45.2), ("45 s d'avance".to_string(), GREEN));
        assert_eq!(delay_words(0.3).0, "À l'heure");
    }

    #[test]
    fn the_sheet_is_the_plugins_json() {
        let j = sheet_json(&snap());
        let v: serde_json::Value = serde_json::from_str(&j).expect("valid JSON");
        assert_eq!(v["action"], "SUBMIT_DRIVER_SHEET");
        assert_eq!(v["driver"], "michael75");
        assert_eq!(v["line"], "21");
        assert_eq!(v["tour"], "2D");
        assert_eq!(v["start_time"], "18:00:00");
        assert_eq!(v["end_time"], "18:40:05");
        assert_eq!(v["distance_km"], 12.3);
        assert_eq!(v["passengers"], 31);
        assert_eq!(v["delay"], "+1:15");
        assert_eq!(v["tickets_sold"], 4);
        // a name with quotes stays JSON
        let odd = Snapshot { driver: "a\"b\\c".into(), ..snap() };
        assert!(serde_json::from_str::<serde_json::Value>(&sheet_json(&odd)).is_ok());
    }

    #[test]
    fn the_website_gets_the_sheet_with_its_date_and_says_what_it_earned() {
        let j = with_saved_at(&sheet_json(&snap()), "2026-10-28 19:45:12");
        let v: serde_json::Value = serde_json::from_str(&j).expect("valid JSON");
        assert_eq!(v["saved_at"], "2026-10-28 19:45:12");
        assert_eq!(v["action"], "SUBMIT_DRIVER_SHEET");
        assert_eq!(read_answer(r#"{"success":true,"earned":12.5}"#), SiteAnswer::Validated("12.5".into()));
        assert_eq!(read_answer(r#"{"success":"true","earned":"8,40"}"#), SiteAnswer::Validated("8,40".into()));
        assert_eq!(read_answer(r#"{"success":1}"#), SiteAnswer::Validated("0".into()));
        assert_eq!(read_answer(r#"{"success":false,"message":"chauffeur inconnu"}"#), SiteAnswer::Refused("chauffeur inconnu".into()));
        assert!(matches!(read_answer("<html>erreur</html>"), SiteAnswer::Refused(_)));
    }

    /// The tablet's place and size, as kept in their file.
    #[test]
    fn the_tablet_keeps_its_place_and_size() {
        let p = TabletPlace::read("1500 220 1.3");
        assert_eq!(p, TabletPlace { at: Some((1500.0, 220.0)), k: 1.3 });
        assert_eq!(TabletPlace::read(&p.text()), p);
        let fresh = TabletPlace::read("- - 1.0");
        assert_eq!(fresh.at, None);
        // never moved: at the right, under the bar, away from the chat on the left
        assert_eq!(fresh.corner(1920.0), (1920.0 - 380.0 - 16.0, 110.0));
        assert_eq!(TabletPlace::read("x y 9").k, TABLET_SIZES.1, "a size out of bounds is held to them");
        // its corner dragged: wider, bigger, within bounds
        assert_eq!(resized(1.0, 38.0), 1.1);
        assert_eq!(resized(1.0, -1000.0), TABLET_SIZES.0);
        assert_eq!(resized(1.5, 1000.0), TABLET_SIZES.1);
        assert_eq!(TabletPlace::read(""), TabletPlace::default());
        let t = tablet(&snap(), (100.0, 200.0), 1.5);
        assert!(t.width == 570.0 && (t.x, t.y) == (100.0, 200.0));
    }

    /// The sheet goes as the terminus is reached, once: not again when the duty moves on.
    #[test]
    fn the_sheet_goes_at_the_terminus_once() {
        let driving = Snapshot { trip_done: false, ..snap() };
        let arrived = Snapshot { trip_done: true, ..snap() };
        assert!(!terminus_reached(None, &driving));
        assert!(terminus_reached(None, &arrived));
        let sent = trip_id(&arrived);
        assert!(!terminus_reached(sent.as_ref(), &arrived), "not twice");
        // the next trip, reached in its turn
        assert!(terminus_reached(sent.as_ref(), &Snapshot { trip: Some(14), trip_done: true, ..snap() }));
        // when the duty moves on, the trip already sent is no trip given up
        assert!(trip_ended(&arrived, &Snapshot { trip: Some(14), trip_done: false, ..snap() }) && trip_id(&arrived) == sent);
    }

    /// A trip ends when the duty goes on to its next trip or ends, not when nothing was driven
    /// or there is no duty.
    #[test]
    fn a_trip_ends_when_the_duty_goes_on() {
        let on_trip = Snapshot { trip: Some(13), ..snap() };
        assert!(trip_ended(&on_trip, &Snapshot { trip: Some(14), ..snap() }));
        assert!(trip_ended(&on_trip, &Snapshot { trip: None, line: None, tour: None, ..snap() }), "the duty ended");
        assert!(trip_ended(&on_trip, &Snapshot { trip: Some(13), tour: Some("3A".into()), ..snap() }), "another duty");
        assert!(!trip_ended(&on_trip, &on_trip.clone()));
        assert!(!trip_ended(&Snapshot { trip: Some(13), km: 0.0, carried: 0, ..snap() }, &Snapshot { trip: Some(14), ..snap() }), "nothing driven");
        assert!(!trip_ended(&Snapshot { trip: None, ..snap() }, &Snapshot { trip: Some(1), ..snap() }), "no duty before");
        // (whether its sheet goes: only with the terminus reached - see stalium_send_trip_sheet)
        assert!(!snap().trip.is_none() && snap().trip_done);
    }

    /// Only a game server on the Stalium machine is a Stalium server.
    #[test]
    fn a_stalium_server_is_one_on_the_stalium_machine() {
        let ips: Vec<std::net::IpAddr> = vec!["203.0.113.7".parse().unwrap()];
        assert!(is_stalium_host("203.0.113.7:27017".parse().unwrap(), &ips));
        assert!(is_stalium_host("[::ffff:203.0.113.7]:27018".parse().unwrap(), &ips), "the same machine seen in IPv6");
        assert!(!is_stalium_host("198.51.100.2:27017".parse().unwrap(), &ips), "another server");
        assert!(!is_stalium_host("127.0.0.1:27017".parse().unwrap(), &ips), "a server on this PC");
        assert!(!is_stalium_host("203.0.113.7:27017".parse().unwrap(), &[]), "a build without the secret");
    }

    #[test]
    fn the_panels_carry_their_buttons() {
        let s = snap();
        let ids = |p: &Panel| {
            fn walk(es: &[Element], out: &mut Vec<String>) {
                for e in es {
                    if let Some(id) = &e.id {
                        out.push(id.clone());
                    }
                    if let Kind::Row { children, .. } | Kind::Box { children, .. } = &e.kind {
                        walk(children, out);
                    }
                }
            }
            let mut out = Vec::new();
            walk(&p.children, &mut out);
            out
        };
        assert!(ids(&tablet(&s, (16.0, 110.0), 1.0)).contains(&"close_tablet".to_string()));
        assert!(ids(&tablet(&s, (16.0, 110.0), 1.0)).contains(&"tablet_resize".to_string()), "its corner to resize it");
        assert!(!ids(&tablet(&s, (16.0, 110.0), 1.0)).iter().any(|i| i.contains("sheet") || i.contains("bigger")), "no road sheet of its own, no -/+");
        // the road sheet is in the tablet
        let tt = format!("{:?}", tablet(&s, (16.0, 110.0), 1.0).children);
        assert!(tt.contains("FEUILLE EN COURS") && tt.contains("Prise de service : 18:00:00") && tt.contains("Distance : 12,3 km") && tt.contains("Voyageurs : 31") && tt.contains("SERVICE 2D"));
        assert!(ids(&bar(&s, 1920.0)).is_empty(), "the bar takes no clicks");
        // the next stop and how far; a narrow screen leaves the gauges out
        let text = |p: &Panel| format!("{:?}", p.children);
        assert!(text(&bar(&s, 1920.0)).contains("1,3 km"));
        assert!(text(&bar(&s, 1920.0)).contains("1:15 de retard"));
        assert!(text(&bar(&s, 1920.0)).contains("Carburant"));
        assert!(!text(&bar(&s, 1100.0)).contains("Carburant"));
        assert!(!text(&bar(&s, 1920.0)).contains("F10"), "the keys are on the tablet, not the bar");
        // as wide as what it shows: out of service it is narrower, never wider than the screen
        let off = Snapshot { line: None, tour: None, trip: None, terminus: None, next_stop: None, next_stop_m: None, delay: None, ..snap() };
        assert!(bar(&off, 1920.0).width < bar(&s, 1920.0).width);
        assert!(!text(&bar(&off, 1920.0)).contains("Feuille"), "out of service, no sheet on the bar");
        assert!(text(&bar(&off, 1920.0)).contains("Carburant"));
        assert!(text(&bar(&s, 1920.0)).contains("Ext. 8 °C • Bus 21 °C"));
        // the game's font draws a raised dot as a hyphen: none on the bar
        assert!(!text(&bar(&s, 1920.0)).contains('·'));
        assert!(text(&bar(&s, 1920.0)).contains("à 1,3 km"));
        assert!(bar(&s, 900.0).width <= 860.0);
    }
}
