//! Stalium Transport's HUD, part of the game (it was a Lua plugin, `fr.stalium.route`, to be
//! copied into the game's folder): a bar along the top of the screen, the driver's tablet and
//! the road sheet.
//!
//! - The bar (always, while the setting `stalium_hud` is on and the player drives): the
//!   driver, the map, the bus, the time, the speed, the kilometres of the sheet, the odometer,
//!   the outside and cabin temperatures, the tank, the passengers aboard, the line and its
//!   terminus, the next stop and the delay. It takes the place of the game's information bar.
//! - The tablet (`stalium_tablet`, F11 by default): the line, the next stop, the delay, the
//!   passengers and the takings.
//! - The road sheet (`stalium_sheet`, F10, to read): the trip since it began (the driver, the
//!   line, the times, the distance, the passengers, the tickets, the delay, the bus), sent by
//!   itself at the end of each trip of the duty (the terminus served: the duty goes on to its
//!   next trip, or ends) to the Stalium website ([`SITE_API`], as Stalium Companion sent it: the plugin's
//!   `SUBMIT_DRIVER_SHEET` JSON and `saved_at`; the site answers `success` and `earned`) and
//!   to the Discord bot on this computer (UDP 127.0.0.1:47800), after which a new sheet
//!   begins. Not to Companion any more (UDP :4040): it sent the sheet on to the site, which
//!   would have had it twice.
//! - `stalium_hud` (F7): the HUD hidden or shown again.
//!
//! The tablet and the sheet take the mouse while open (Esc closes them); the bar does not.

use super::*;
use omsi_plugin::ui::Anchor;

/// What the HUD shows of the plugin's version: the HUD's own.
const VERSION: &str = "v3.0";
/// The Discord bot's / Stalium Live's port on this computer.
pub(crate) const BOT_PORT: u16 = 47800;
/// Where the road sheets go on the Stalium website (Stalium Companion's `api_url`).
pub(crate) const SITE_API: &str = "https://omsi.stalium-transport.fr/api/omsi_trips.php";

const ORANGE: Rgba = Rgba([0xF4, 0x7F, 0x30, 255]);
const GREY: Rgba = Rgba([0x7F, 0x8C, 0x8D, 255]);
const LIGHT: Rgba = Rgba([0xBD, 0xC3, 0xC7, 255]);
const BLUE: Rgba = Rgba([0x34, 0x98, 0xDB, 255]);
const YELLOW: Rgba = Rgba([0xF1, 0xC4, 0x0F, 255]);
const GREEN: Rgba = Rgba([0x2E, 0xCC, 0x71, 255]);
const RED: Rgba = Rgba([0xE7, 0x4C, 0x3C, 255]);
const WHITE: Rgba = Rgba([255, 255, 255, 255]);

/// The HUD's state in the game's panels.
#[derive(Default)]
pub(crate) struct StaliumHud {
    pub(crate) tablet: bool,
    pub(crate) sheet: bool,
    /// Hidden with `stalium_hud` (F7): nothing shows, the game's information bar neither.
    pub(crate) hidden: bool,
    /// The sheet's beginning: the game time (s), the session's metres, its tickets (count,
    /// takings) and passengers then. None until the first frame with a bus.
    start: Option<SheetStart>,
    /// The last frame's values: the sheet of a trip that has just ended is that.
    last: Option<Snapshot>,
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
    pub keys: [String; 3],
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
    pub terminus: Option<String>,
    pub next_stop: Option<String>,
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
fn send_sheet(json: &str) {
    if let Ok(socket) = std::net::UdpSocket::bind(("127.0.0.1", 0)) {
        let _ = socket.send_to(json.as_bytes(), ("127.0.0.1", BOT_PORT));
    }
    let body = with_saved_at(json, &saved_at_now());
    let spawned = std::thread::Builder::new().name("stalium-sheet".into()).spawn(move || {
        let agent = ureq::AgentBuilder::new().timeout_connect(std::time::Duration::from_secs(6)).timeout(std::time::Duration::from_secs(12)).build();
        let answer = match agent.post(SITE_API).set("Content-Type", "application/json").set("User-Agent", "openOMSI-Stalium").send_string(&body) {
            Ok(r) => read_answer(&r.into_string().unwrap_or_default()),
            // (the site's own refusal, with its words)
            Err(ureq::Error::Status(code, r)) => SiteAnswer::Refused(format!("HTTP {code} {}", short(&r.into_string().unwrap_or_default(), 60))),
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

fn driver_name(app: &crate::App) -> String {
    if let Some(l) = app.net.lan.as_ref().filter(|l| !l.my_name.trim().is_empty()) {
        return l.my_name.trim().to_string();
    }
    // (BCS's personnel file is "Bus Company Simulator" for every player: not a name)
    let usable = |n: &str| !n.is_empty() && n != "Driver" && !n.eq_ignore_ascii_case("bbs") && !n.eq_ignore_ascii_case("Bus Company Simulator");
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
        let keys = [key("stalium_tablet"), key("stalium_sheet"), key("stalium_hud")];
        let on_server = stalium_server_name(self);
        let mut s = Snapshot {
            keys,
            stalium_server: on_server.is_some(),
            // (on a Stalium server, the name the server knows the player by)
            driver: on_server.unwrap_or_else(|| driver_name(self)),
            map: map.trim().to_string(),
            bus: if bus.is_empty() { def.path.file_stem().map(|f| f.to_string_lossy().to_string()).unwrap_or_default() } else { bus },
            number: v.number().trim().to_string(),
            time: self.clock.time,
            speed_kmh: v.physics.velocity_kmh().abs(),
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
                s.terminus = Some(trip.terminus.trim().to_string()).filter(|t| !t.is_empty());
                s.next_stop = trip.stops.get(d.next_stop).map(|st| st.name.trim().to_string()).filter(|n| !n.is_empty());
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

    /// The road sheet opened or closed (`stalium_sheet`).
    pub(crate) fn stalium_toggle_sheet(&mut self) {
        let h = &mut self.menus.game_panels.stalium;
        h.sheet = !h.sheet;
        if h.sheet {
            h.hidden = false;
        }
    }

    /// The HUD hidden or shown again (`stalium_hud`): hidden, its windows close too.
    pub(crate) fn stalium_toggle_hud(&mut self) {
        let h = &mut self.menus.game_panels.stalium;
        h.hidden = !h.hidden;
        if h.hidden {
            h.tablet = false;
            h.sheet = false;
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
        if !s.stalium_server {
            // (alone, hosting or on another server: the sheet stays here, a new one begins)
            log::info!("Stalium: trip {:?} of line {:?} ended off the Stalium servers: no road sheet sent", s.trip, s.line);
            self.menus.game_panels.stalium.start = None;
            self.service_msg = Some(("Course terminée : hors serveur Stalium, la feuille de route n'est pas envoyée".into(), 5.0));
            return;
        }
        let json = sheet_json(s);
        log::info!("Stalium: trip {:?} of line {:?} ended: road sheet {json}", s.trip, s.line);
        send_sheet(&json);
        self.menus.game_panels.stalium.start = None;
        self.service_msg = Some((format!("Course terminée : feuille de route {} envoyée au site Stalium ({:.1} km, {} voyageur(s))...", s.line.as_deref().unwrap_or("HLP"), s.km, s.carried), 6.0));
    }
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

fn t(s: &str, c: Rgba) -> Element {
    colored(el(None, Kind::Text { text: s.to_string(), size: 13.0, weight: Weight::Regular, align: Align::Left, wrap: false }), c)
}

fn small(s: &str) -> Element {
    colored(el(None, Kind::Text { text: s.to_string(), size: 11.0, weight: Weight::Regular, align: Align::Left, wrap: true }), Rgba([0x9C, 0xA3, 0xAF, 255]))
}

fn icon(name: &str, c: Rgba) -> Element {
    colored(el(None, Kind::Icon { name: name.to_string(), size: 16.0 }), c)
}

fn badge(s: &str, c: Rgba) -> Element {
    colored(el(None, Kind::Badge { text: s.to_string(), text_color: Some(WHITE), weight: Weight::Medium }), c)
}

fn route_text(s: &Snapshot) -> String {
    match (s.line.as_deref(), s.terminus.as_deref()) {
        (Some(l), Some(d)) => format!("{} > {}", short(l, 8), short(d, 18)),
        (Some(l), None) => format!("Ligne {}", short(l, 8)),
        (None, Some(d)) => format!("Dest : {}", short(d, 18)),
        (None, None) => "Sans service".into(),
    }
}

/// The bar along the top: `wide` (a logical width of 1650 or more) shows the vehicle's details
/// and the next stop too.
pub(crate) fn bar(s: &Snapshot, width: f32) -> Panel {
    let wide = width >= 1650.0;
    let sep = divider;
    let bus = if s.number.is_empty() { short(&s.bus, 18) } else { format!("{} #{}", short(&s.bus, 14), s.number) };
    let mut row = vec![
        colored(el(None, Kind::Text { text: "STALIUM".into(), size: 13.0, weight: Weight::Bold, align: Align::Left, wrap: false }), ORANGE),
        t(VERSION, GREY),
        t(&short(&s.driver, 16), WHITE),
        sep(),
        icon("map", BLUE),
        t(&short(&s.map, 16), BLUE),
        sep(),
        icon("directions_bus", ORANGE),
        t(&bus, YELLOW),
        sep(),
        t(&clock(s.time), WHITE),
        sep(),
        t(&format!("{:.0} km/h", s.speed_kmh), WHITE),
        sep(),
        t(&format!("{:.1} km", s.km), GREEN),
    ];
    if wide {
        if let Some(odo) = s.odometer_km {
            row.extend([sep(), t(&format!("Odo {} km", odo.floor() as i64), LIGHT)]);
        }
        row.extend([sep(), t(&format!("EXT {:.0}° / INT {:.0}°", s.temperatures.0, s.temperatures.1), LIGHT)]);
        if let Some(f) = s.fuel {
            row.extend([sep(), t(&format!("Réservoir {:.0} %", f * 100.0), LIGHT)]);
        }
        let pax = match s.capacity {
            Some(c) => format!("{}/{} pax", s.aboard, c),
            None => format!("{} pax", s.aboard),
        };
        row.extend([sep(), t(&pax, LIGHT)]);
    }
    let route = route_text(s);
    let route_color = if route == "Sans service" { GREY } else { BLUE };
    row.extend([sep(), t(&route, route_color)]);
    if wide {
        if let Some(n) = s.next_stop.as_deref() {
            row.extend([sep(), t(&format!("Prochain : {}", short(n, 18)), Rgba([0xE0, 0xE0, 0xE0, 255]))]);
        }
    }
    if let Some(d) = s.delay {
        let (text, c) = delay_text(d);
        row.extend([sep(), badge(&text, c)]);
    }
    row.extend([sep(), small(&format!("{} Feuille · {} Tablette · {} HUD", s.keys[1], s.keys[0], s.keys[2]))]);
    let mut p = card(Anchor::Top, 0.0, 8.0, (width - 40.0).clamp(600.0, 1880.0), 238, vec![row_of(row)]);
    p.padding = 6.0;
    p.radius = 8.0;
    p.background = Some(Rgba([0x0F, 0x11, 0x16, 238]));
    p
}

fn row_of(children: Vec<Element>) -> Element {
    el(None, Kind::Row { children, gap: 8.0, align: RowAlign::Center })
}

/// The driver's tablet.
pub(crate) fn tablet(s: &Snapshot) -> Panel {
    let line = s.line.clone().unwrap_or_else(|| "DÉPÔT".into());
    let (delay, delay_color) = s.delay.map(delay_text).unwrap_or_else(|| ("--".into(), GREY));
    let pax = match s.capacity {
        Some(c) => format!("Passagers : {}/{}", s.aboard, c),
        None => format!("Passagers : {}", s.aboard),
    };
    let children = vec![
        row_of(vec![
            icon("directions_bus", Rgba([0x3B, 0x82, 0xF6, 255])),
            Element { grow: true, ..colored(el(None, Kind::Text { text: "STALIUM TRANSPORT".into(), size: 14.0, weight: Weight::Bold, align: Align::Left, wrap: false }), WHITE) },
            badge(&format!("LIGNE {}", short(&line, 10)), Rgba([0x3B, 0x82, 0xF6, 255])),
            button("close_tablet", "X", None),
        ]),
        divider(),
        row_of(vec![Element { grow: true, ..t(&format!("Prochain : {}", s.next_stop.as_deref().unwrap_or("En attente")), WHITE) }, badge(&delay, delay_color)]),
        row_of(vec![Element { grow: true, ..t(&format!("Terminus : {}", s.terminus.as_deref().unwrap_or("Sans service")), WHITE) }]),
        row_of(vec![Element { grow: true, ..t(&pax, WHITE) }, badge(&format!("Caisse : {:.2} €", s.tickets.1), Rgba([0x10, 0xB9, 0x81, 255]))]),
        divider(),
        row_of(vec![button("open_sheet", &format!("Feuille de route ({})", s.keys[1]), Some("description"))]),
        small(&format!("{} : feuille de route · {} : tablette · {} : masquer le HUD · Échap : fermer", s.keys[1], s.keys[0], s.keys[2])),
    ];
    let mut p = card(Anchor::TopLeft, 16.0, 64.0, 380.0, 250, children);
    p.accent = Some(Rgba([0x3B, 0x82, 0xF6, 255]));
    p
}

/// The road sheet.
pub(crate) fn sheet(s: &Snapshot) -> Panel {
    let pair = |a: String, ca: Rgba, b: String, cb: Rgba| grow_row(vec![t(&a, ca), t(&b, cb)]);
    let (delay, delay_color) = s.delay.map(delay_text).unwrap_or_else(|| ("--".into(), GREY));
    let bus = if s.number.is_empty() { s.bus.clone() } else { format!("{} #{}", s.bus, s.number) };
    let children = vec![
        colored(title(&format!("FEUILLE DE ROUTE — STALIUM TRANSPORT ({VERSION})")), ORANGE),
        divider(),
        pair(format!("Chauffeur : {}", s.driver), WHITE, format!("Réseau : {}", s.map), LIGHT),
        pair(format!("Ligne : {} (service {})", s.line.as_deref().unwrap_or("HLP"), s.tour.as_deref().unwrap_or("--")), BLUE, format!("Destination : {}", s.terminus.as_deref().unwrap_or("Sans service")), WHITE),
        pair(format!("Prise de service : {}", clock(s.start)), Rgba([0xE0, 0xE0, 0xE0, 255]), format!("Heure actuelle : {}", clock(s.time)), Rgba([0xE0, 0xE0, 0xE0, 255])),
        pair(format!("Distance : {:.1} km", s.km), GREEN, format!("Voyageurs : {} transporté(s)", s.carried), GREEN),
        pair(format!("Billetterie : {} ticket(s) ({:.2} €)", s.tickets.0, s.tickets.1), BLUE, format!("Véhicule : {bus}"), YELLOW),
        row_of(vec![t("Ponctualité :", WHITE), badge(&delay, delay_color)]),
        divider(),
        row_of(vec![button("close_sheet", "Fermer", None)]),
        if s.stalium_server {
            small("Sur le serveur Stalium : la feuille part toute seule sur stalium-transport.fr (et au bot Discord de ce PC) à la fin de chaque course ; une nouvelle feuille commence ensuite.")
        } else {
            colored(small("Hors serveur Stalium (solo, partie hébergée ou autre serveur) : la feuille n'est pas envoyée."), GREY)
        },
    ];
    card(Anchor::Center, 0.0, 0.0, 720.0, 250, children)
}

/// The HUD's panels this frame: (id, the panel or None to take it away).
pub(crate) fn panels(app: &mut crate::App, screen_width: f32) -> Vec<(&'static str, Option<Panel>)> {
    say_answers(app);
    // (the trips' sheets go by themselves, the HUD shown or hidden)
    let mut snap = if app.stalium_hud_on() { app.stalium_snapshot() } else { None };
    if let Some(now) = snap.as_ref() {
        let last = app.menus.game_panels.stalium.last.take();
        if let Some(last) = last.filter(|l| trip_ended(l, now)) {
            app.stalium_send_trip_sheet(&last);
            snap = app.stalium_snapshot();
        }
        app.menus.game_panels.stalium.last = snap.clone();
    }
    let on = app.stalium_hud_on() && !app.menus.game_panels.stalium.hidden;
    let snap = snap.filter(|_| on);
    let h = &app.menus.game_panels.stalium;
    let Some(s) = snap else {
        return vec![("st_bar", None), ("st_tablet", None), ("st_sheet", None)];
    };
    vec![
        ("st_bar", Some(bar(&s, screen_width))),
        ("st_tablet", h.tablet.then(|| tablet(&s))),
        ("st_sheet", h.sheet.then(|| sheet(&s))),
    ]
}

/// A click on one of the HUD's panels.
pub(crate) fn click(app: &mut crate::App, panel: &str, element: &str) {
    match (panel, element) {
        ("st_tablet", "close_tablet") => app.menus.game_panels.stalium.tablet = false,
        ("st_tablet", "open_sheet") => app.menus.game_panels.stalium.sheet = true,
        ("st_sheet", "close_sheet") => app.menus.game_panels.stalium.sheet = false,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap() -> Snapshot {
        Snapshot {
            keys: ["F11".into(), "F10".into(), "F7".into()],
            stalium_server: true,
            driver: "michael75".into(),
            map: "Saint-Servan".into(),
            bus: "Irisbus Citelis 18".into(),
            number: "1351".into(),
            time: 18.0 * 3600.0 + 40.0 * 60.0 + 5.0,
            speed_kmh: 32.4,
            km: 12.34,
            odometer_km: Some(1_067_042.3),
            temperatures: (8.0, 21.0),
            fuel: Some(0.77),
            aboard: 12,
            capacity: Some(87),
            line: Some("21".into()),
            tour: Some("2D".into()),
            trip: Some(13),
            terminus: Some("P+R Ronoe".into()),
            next_stop: Some("Thiers - Victor Hugo".into()),
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
        assert!(ids(&tablet(&s)).contains(&"close_tablet".to_string()));
        assert!(ids(&tablet(&s)).contains(&"open_sheet".to_string()));
        assert_eq!(ids(&sheet(&s)), vec!["close_sheet".to_string()], "the sheet goes by itself: no button to send it");
        assert!(ids(&bar(&s, 1920.0)).is_empty(), "the bar takes no clicks");
        // a narrow screen leaves the vehicle's details out
        let text = |p: &Panel| format!("{:?}", p.children);
        assert!(text(&bar(&s, 1920.0)).contains("Odo"));
        assert!(!text(&bar(&s, 1280.0)).contains("Odo"));
    }
}
