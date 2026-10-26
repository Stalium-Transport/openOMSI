//! Stalium Transport: two players swap their buses (the player list's "Échanger nos bus").
//!
//! A player asks another from the player list; the other accepts or refuses there within
//! [`ANSWER_SECS`]. Accepted, each game takes over the other's bus where it stands - its
//! file, paint and every script variable and string as the other's game sent them (the copy
//! drawn here) - in the place of its own, and the riders of that bus in their places (as the
//! shared world showed them here, see `LanWorld::stalium_riders_of`); its own bus goes with
//! its riders, who ride on in the other game. The duty (line, tour) stays each player's.
//!
//! Allowed only between two buses that stand still (under [`STILL_KMH`]) within [`NEAR_M`]
//! of each other, driven (not on foot), not on a trip started by BCS (its record of the trip
//! - km, fuel, tickets - is the bus's), and when the other's bus is installed here as itself
//! (not a stand-in nor a copy fetched from the server only to be drawn) and offered by the
//! server. Each game checks its own side again before it answers or swaps.
//!
//! In two steps, so that both games check the rules at the last moment: the asked game says
//! yes and waits; the asking game checks everything again and then says go and swaps; the
//! asked game swaps on that go (or, without it within [`GO_SECS`], not at all).
//!
//! The messages go through the session's player commands (`LanSession::command`, through the
//! server), which may be lost: each is sent [`REPEATS`] times and taken once
//! (`stalium_swap ask|yes|go|no|cancel <nonce> [reason]`). A game without this answers
//! nothing: the request runs out.

use crate::App;
use std::sync::Mutex;

/// How long the asked player has to answer (s); the asker waits a little longer.
pub(crate) const ANSWER_SECS: f32 = 30.0;
/// Both buses stand still: slower than this (km/h).
pub(crate) const STILL_KMH: f32 = 1.0;
/// The two buses at most this far apart (m): riders are shared only near (`CAR_RADIUS`), and
/// the drivers meet to swap.
pub(crate) const NEAR_M: f64 = 200.0;
/// How long the asked game waits for the go after its yes (s).
pub(crate) const GO_SECS: f32 = 10.0;
/// Each message is sent this many times, [`REPEAT_GAP`] seconds apart.
const REPEATS: usize = 3;
const REPEAT_GAP: f32 = 0.4;

/// A message between two games.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Msg {
    Ask(u64),
    Yes(u64),
    Go(u64),
    No(u64, String),
    Cancel(u64),
}

impl Msg {
    pub(crate) fn text(&self) -> String {
        match self {
            Msg::Ask(n) => format!("stalium_swap ask {n}"),
            Msg::Yes(n) => format!("stalium_swap yes {n}"),
            Msg::Go(n) => format!("stalium_swap go {n}"),
            Msg::No(n, why) => format!("stalium_swap no {n} {}", why.trim()).trim_end().to_string(),
            Msg::Cancel(n) => format!("stalium_swap cancel {n}"),
        }
    }

    /// What follows `stalium_swap ` in a command.
    pub(crate) fn parse(rest: &str) -> Option<Msg> {
        let mut f = rest.trim().splitn(3, ' ');
        let verb = f.next()?;
        let nonce = f.next()?.parse::<u64>().ok().filter(|n| *n != 0)?;
        let why = f.next().unwrap_or("").trim().to_string();
        match verb {
            "ask" => Some(Msg::Ask(nonce)),
            "yes" => Some(Msg::Yes(nonce)),
            "go" => Some(Msg::Go(nonce)),
            "no" => Some(Msg::No(nonce, why)),
            "cancel" => Some(Msg::Cancel(nonce)),
            _ => None,
        }
    }

    fn nonce(&self) -> u64 {
        match self {
            Msg::Ask(n) | Msg::Yes(n) | Msg::Go(n) | Msg::No(n, _) | Msg::Cancel(n) => *n,
        }
    }

    fn verb(&self) -> &'static str {
        match self {
            Msg::Ask(_) => "ask",
            Msg::Yes(_) => "yes",
            Msg::Go(_) => "go",
            Msg::No(..) => "no",
            Msg::Cancel(_) => "cancel",
        }
    }
}

/// A request one way: to or from player `who`, `since` seconds of [`State::clock`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Pending {
    pub(crate) who: u32,
    pub(crate) name: String,
    pub(crate) nonce: u64,
    pub(crate) since: f32,
}

#[derive(Default)]
pub(crate) struct State {
    clock: f32,
    /// Our request, waiting for its answer.
    pub(crate) asked: Option<Pending>,
    /// A request to us, waiting for ours.
    pub(crate) offer: Option<Pending>,
    /// A request to us we said yes to, waiting for the go.
    pub(crate) accepted: Option<Pending>,
    /// Messages still to be sent again: (when, to whom, text).
    queue: Vec<(f32, u32, String)>,
    /// Messages taken already (sender, verb, nonce): their repeats are dropped.
    seen: Vec<(u32, &'static str, u64)>,
}

impl State {
    fn send(&mut self, to: u32, msg: &Msg) -> String {
        let text = msg.text();
        for k in 1..REPEATS {
            self.queue.push((self.clock + REPEAT_GAP * k as f32, to, text.clone()));
        }
        text
    }

    /// Is this message new (and remember it)?
    fn first(&mut self, from: u32, msg: &Msg) -> bool {
        let key = (from, msg.verb(), msg.nonce());
        if self.seen.contains(&key) {
            return false;
        }
        if self.seen.len() >= 64 {
            self.seen.remove(0);
        }
        self.seen.push(key);
        true
    }
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(State::default))
}

/// The request to us now, for the player list: (player id, name).
pub(crate) fn offer() -> Option<(u32, String)> {
    with(|s| s.offer.as_ref().map(|o| (o.who, o.name.clone())))
}

/// The player we asked and are waiting for.
pub(crate) fn asked() -> Option<u32> {
    with(|s| s.asked.as_ref().map(|a| a.who))
}

fn nonce() -> u64 {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
    (t ^ (std::process::id() as u64).rotate_left(32)).max(1)
}

fn send(app: &mut App, to: u32, msg: &Msg) {
    let text = with(|s| s.send(to, msg));
    if let Some(l) = app.net.lan.as_mut() {
        l.command(to, &text);
    }
}

fn name_of(app: &App, who: u32) -> String {
    app.net.remotes.remotes.get(&who).map(|r| r.name.clone()).filter(|n| !n.trim().is_empty()).unwrap_or_else(|| format!("joueur #{who}"))
}

/// Why we may not swap with player `who` now (None: we may). Each game checks its own side
/// (its bus, its BCS trip) and the distance between the two: asking, answering, and again
/// just before the go (`receive`).
pub(crate) fn why_not(app: &App, who: u32) -> Option<String> {
    let Some(lan) = app.net.lan.as_ref() else { return Some("pas de partie multijoueur".into()) };
    if who == lan.my_id {
        return Some("c'est vous".into());
    }
    let Some(p) = app.player.as_ref().filter(|_| app.session.on_foot.is_none() && app.net.inside_remote.is_none()) else {
        return Some("vous ne conduisez pas de bus".into());
    };
    if crate::stalium_bcs::started_by_bcs(&app.args) {
        return Some("pas pendant un trajet BCS".into());
    }
    if app.menus.pending_placement.is_some() {
        return Some("un véhicule est déjà en cours de chargement".into());
    }
    if p.vehicle.physics.velocity_kmh().abs() >= STILL_KMH {
        return Some("votre bus doit être à l'arrêt".into());
    }
    let Some(rv) = app.net.remotes.remotes.get(&who) else { return Some("son bus n'est pas encore arrivé ici".into()) };
    if rv.last.speed_kmh.abs() >= STILL_KMH {
        return Some("son bus doit être à l'arrêt".into());
    }
    if (rv.vehicle().position - p.vehicle.position).truncate().length() > NEAR_M {
        return Some(format!("vos bus doivent être à moins de {NEAR_M:.0} m l'un de l'autre"));
    }
    let store = omsi_launcher_lib::data_dir().join("lan-vehicles");
    if rv.stand_in || rv.fallback || rv.last.bus.trim().is_empty() || rv.vehicle().ty.def.path.starts_with(&store) {
        return Some("son bus n'est pas installé ici".into());
    }
    if crate::lan::server_offers().is_some_and(|o| !crate::lan::offers(&o, &rv.last.bus)) {
        return Some("le serveur ne propose pas son bus".into());
    }
    None
}

/// The player list's "Échanger nos bus" with player `who`.
pub(crate) fn ask(app: &mut App, who: u32) {
    if let Some(why) = why_not(app, who) {
        app.service_msg = Some((format!("Échange impossible : {why}"), 5.0));
        return;
    }
    // (one request at a time: a new one replaces ours)
    if let Some(old) = with(|s| s.asked.take()) {
        send(app, old.who, &Msg::Cancel(old.nonce));
    }
    let n = nonce();
    let name = name_of(app, who);
    with(|s| s.asked = Some(Pending { who, name: name.clone(), nonce: n, since: s.clock }));
    send(app, who, &Msg::Ask(n));
    log::info!("Stalium: we ask player {who} ({name}) to swap buses ({n})");
    app.service_msg = Some((format!("Demande d'échange envoyée à {name} : en attente de sa réponse"), 6.0));
}

/// Our request withdrawn (the player list).
pub(crate) fn cancel(app: &mut App) {
    if let Some(a) = with(|s| s.asked.take()) {
        send(app, a.who, &Msg::Cancel(a.nonce));
        app.service_msg = Some((format!("Demande d'échange à {} annulée", a.name), 4.0));
    }
}

/// The player list's answer to the request to us.
pub(crate) fn answer(app: &mut App, accept: bool) {
    let Some(o) = with(|s| s.offer.take()) else { return };
    if !accept {
        send(app, o.who, &Msg::No(o.nonce, "refusé".into()));
        app.service_msg = Some((format!("Échange avec {} refusé", o.name), 4.0));
        return;
    }
    if let Some(why) = why_not(app, o.who) {
        send(app, o.who, &Msg::No(o.nonce, why.clone()));
        app.service_msg = Some((format!("Échange impossible : {why}"), 6.0));
        return;
    }
    // (the swap waits for their go: their game checks everything again first)
    send(app, o.who, &Msg::Yes(o.nonce));
    log::info!("Stalium: we accept player {}'s swap ({}): waiting for their go", o.who, o.nonce);
    app.service_msg = Some((format!("Échange accepté : en attente de la confirmation de {}...", o.name), GO_SECS));
    with(|s| s.accepted = Some(Pending { since: s.clock, ..o }));
}

/// A `stalium_swap …` command from player `from`.
pub(crate) fn receive(app: &mut App, from: u32, rest: &str) {
    let Some(msg) = Msg::parse(rest) else {
        log::info!("Stalium: swap message '{rest}' from player {from} not understood");
        return;
    };
    if !with(|s| s.first(from, &msg)) {
        return;
    }
    let name = name_of(app, from);
    match msg {
        Msg::Ask(n) => {
            // (refused at once where we could not: no question asked for nothing)
            if let Some(why) = why_not(app, from) {
                log::info!("Stalium: player {from} asks to swap buses: no, {why}");
                send(app, from, &Msg::No(n, why));
                return;
            }
            if with(|s| s.offer.as_ref().is_some_and(|o| o.who != from)) {
                send(app, from, &Msg::No(n, "une autre demande est en cours".into()));
                return;
            }
            log::info!("Stalium: player {from} ({name}) asks to swap buses ({n})");
            with(|s| s.offer = Some(Pending { who: from, name: name.clone(), nonce: n, since: s.clock }));
            app.service_msg = Some((format!("{name} propose d'échanger vos bus : menu du jeu, Liste des joueurs, pour accepter ou refuser ({ANSWER_SECS:.0} s)"), ANSWER_SECS));
        }
        Msg::Yes(n) => {
            let ours = with(|s| match s.asked.take() {
                Some(a) if a.who == from && a.nonce == n => Some(a),
                other => {
                    s.asked = other;
                    None
                }
            });
            let Some(a) = ours else {
                log::info!("Stalium: player {from} accepts a swap we no longer ask for ({n}): ignored");
                return;
            };
            // (the rules once more, at the last moment: they swap only on our go)
            if let Some(why) = why_not(app, from) {
                log::info!("Stalium: player {from} accepted the swap, but now {why}: called off");
                send(app, from, &Msg::Cancel(n));
                app.service_msg = Some((format!("Échange annulé : {why}"), 6.0));
                return;
            }
            send(app, from, &Msg::Go(n));
            take_over(app, from, &a.name);
        }
        Msg::Go(n) => {
            let ours = with(|s| match s.accepted.take() {
                Some(o) if o.who == from && o.nonce == n => Some(o),
                other => {
                    s.accepted = other;
                    None
                }
            });
            let Some(o) = ours else {
                log::info!("Stalium: player {from} says go for a swap we are not waiting for ({n}): ignored");
                return;
            };
            // (they have checked everything and swap now: ours goes too, else both of us
            // would drive their bus)
            take_over(app, from, &o.name);
        }
        Msg::No(n, why) => {
            let ours = with(|s| match s.asked.take() {
                Some(a) if a.who == from && a.nonce == n => true,
                other => {
                    s.asked = other;
                    false
                }
            });
            if ours {
                log::info!("Stalium: player {from} does not swap buses: {why}");
                let why = if why.is_empty() { "refusé".to_string() } else { why };
                app.service_msg = Some((format!("{name} n'échange pas : {why}"), 6.0));
            }
        }
        Msg::Cancel(n) => {
            let ours = with(|s| {
                let mine = |p: &Option<Pending>| p.as_ref().is_some_and(|o| o.who == from && o.nonce == n);
                let hit = mine(&s.offer) || mine(&s.accepted);
                if mine(&s.offer) {
                    s.offer = None;
                }
                if mine(&s.accepted) {
                    s.accepted = None;
                }
                hit
            });
            if ours {
                app.service_msg = Some((format!("{name} a annulé sa demande d'échange"), 4.0));
            }
        }
    }
}

/// Once a frame (LAN play): the repeats, and the requests that ran out.
pub(crate) fn tick(app: &mut App, dt: f32) {
    let (due, expired_ask, expired_offer, expired_go) = with(|s| {
        s.clock += dt.max(0.0);
        let now = s.clock;
        let due: Vec<(u32, String)> = s.queue.iter().filter(|q| q.0 <= now).map(|q| (q.1, q.2.clone())).collect();
        s.queue.retain(|q| q.0 > now);
        let expired_ask = s.asked.take_if(|a| now - a.since > ANSWER_SECS + 5.0);
        let expired_offer = s.offer.take_if(|o| now - o.since > ANSWER_SECS);
        let expired_go = s.accepted.take_if(|o| now - o.since > GO_SECS);
        (due, expired_ask, expired_offer, expired_go)
    });
    if let Some(l) = app.net.lan.as_mut() {
        for (to, text) in &due {
            l.command(*to, text);
        }
    }
    if let Some(a) = expired_ask {
        send(app, a.who, &Msg::Cancel(a.nonce));
        app.service_msg = Some((format!("{} n'a pas répondu à la demande d'échange", a.name), 5.0));
    }
    if let Some(o) = expired_go {
        log::info!("Stalium: no go from player {} for the swap ({}): not swapped", o.who, o.nonce);
        app.service_msg = Some((format!("L'échange avec {} n'a pas abouti", o.name), 5.0));
    }
    if let Some(o) = expired_offer {
        send(app, o.who, &Msg::No(o.nonce, "pas de réponse".into()));
        app.service_msg = Some((format!("Demande d'échange de {} expirée", o.name), 4.0));
    }
}

/// The session ended: nothing pending any more.
pub(crate) fn reset() {
    with(|s| *s = State::default());
}

/// Take over player `who`'s bus where it stands, as their game sent it, with its riders.
fn take_over(app: &mut App, who: u32, name: &str) {
    let Some(rv) = app.net.remotes.remotes.get(&who) else {
        app.service_msg = Some((format!("Échange impossible : le bus de {name} n'est plus là"), 5.0));
        return;
    };
    let v = rv.vehicle();
    let bus = rv.last.bus.replace('\\', "/");
    let paint = Some(rv.last.paint.clone()).filter(|p| !p.trim().is_empty());
    let (x, y, z, heading) = (v.position.x, v.position.y, v.position.z, v.heading);
    let program = &v.ty.program;
    let vars: Vec<(String, f32)> = program.var_names.iter().zip(&v.state.vars).filter(|(_, x)| x.is_finite()).map(|(n, x)| (n.clone(), *x)).collect();
    let strings: Vec<(String, String)> = program.str_var_names.iter().zip(&v.state.str_vars).map(|(n, s)| (n.clone(), s.clone())).collect();
    let riders = app.net.lan_world.stalium_riders_of(who);
    log::info!("Stalium: swap with player {who} ({name}): taking over {bus} at ({x:.1}, {y:.1}), {} variables, {} strings, {} riders", vars.len(), strings.len(), riders.len());
    let one = crate::cli::Args {
        bus: Some(bus.clone()),
        spawn: Some(format!("{x},{y},{heading},{z}")),
        situation: None,
        situation_vars: vars,
        situation_strvars: strings,
        situation_odometer_km: None,
        situation_others: Vec::new(),
        line: None,
        tour: None,
        trip: None,
        autostart: false,
        paint,
        ..app.args.clone()
    };
    let replace = app.player.as_ref().map(|p| p.uid);
    let title = app.menus.vehicle_list.iter().find(|v| v.1 == bus).map(|v| v.0.clone()).unwrap_or_else(|| bus.clone());
    app.start_placement(one, title, replace, heading, Some((who, riders)));
    app.service_msg = Some((format!("Échange avec {name} : chargement de son bus..."), 6.0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_read_back() {
        for m in [Msg::Ask(7), Msg::Yes(7), Msg::Go(7), Msg::No(7, "votre bus doit être à l'arrêt".into()), Msg::No(9, String::new()), Msg::Cancel(12)] {
            let text = m.text();
            let rest = text.strip_prefix("stalium_swap ").unwrap();
            assert_eq!(Msg::parse(rest), Some(m), "{text}");
            assert!(text.chars().count() <= omsi_net::MAX_CHAT && !text.contains('|'), "{text}");
        }
        assert_eq!(Msg::parse("ask 0"), None);
        assert_eq!(Msg::parse("ask x"), None);
        assert_eq!(Msg::parse("swap 3"), None);
        assert_eq!(Msg::parse(""), None);
    }

    #[test]
    fn a_message_is_sent_three_times_and_taken_once() {
        let mut s = State::default();
        s.send(4, &Msg::Ask(5));
        assert_eq!(s.queue.len(), REPEATS - 1);
        assert!(s.first(4, &Msg::Ask(5)));
        assert!(!s.first(4, &Msg::Ask(5)));
        assert!(s.first(4, &Msg::Yes(5)));
        assert!(s.first(3, &Msg::Ask(5)));
        for k in 0..100 {
            s.first(9, &Msg::Cancel(100 + k));
        }
        assert!(s.seen.len() <= 64);
    }
}
