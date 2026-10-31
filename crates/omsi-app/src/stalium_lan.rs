//! Stalium: where a LAN player is, for the host that keeps the world around the players.
//!
//! A player who parked the bus and walked into another player's bus as a passenger still
//! sends the parked bus as the pose's place; where the player walks and the bus the player
//! sits in come apart (the pose's walker and its `aboard`). The host kept the traffic, the
//! people at the stops and the other players' passengers within `CAR_RADIUS` of the parked bus:
//! once the bus being ridden in was 650 m from it, all its passengers vanished for the rider
//! (Horizon16, 0.3.45). The player is where `omsi_lan::server_cfg::player_info` says: in the
//! bus ridden in, else where the player walks, else in the bus driven.

use glam::DVec3;

/// Where player `q` is and which way it looks (heading, degrees), or None when it is nowhere
/// (no bus and not on foot). `bus_of`: where a player's bus is and its heading (the bus a
/// passenger rides in).
pub(crate) fn player_at(q: &omsi_net::Pose, bus_of: impl Fn(u32) -> Option<(DVec3, f32)>) -> Option<(DVec3, f32)> {
    match q.walker {
        Some(w) => match w.aboard.and_then(|a| bus_of(a.owner)) {
            Some(bus) => Some(bus),
            None => Some((DVec3::new(w.x, w.y, w.z), w.heading)),
        },
        None if q.has_vehicle() => Some((DVec3::new(q.x, q.y, q.z), q.heading)),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose(id: u32, at: (f64, f64)) -> omsi_net::Pose {
        omsi_net::Pose { id, x: at.0, y: at.1, flags: omsi_net::FLAG_VEHICLE, bus: "Vehicles/MAN_NL/MAN.bus".into(), ..Default::default() }
    }

    fn walker(at: (f64, f64), aboard: Option<u32>) -> omsi_net::Walker {
        omsi_net::Walker { x: at.0, y: at.1, aboard: aboard.map(|owner| omsi_net::Aboard { owner, local: [0.0; 3], seat: None }), ..Default::default() }
    }

    /// Michael parked at (0, 0) and rides in Alex's bus, now 662 m away: he is in Alex's bus.
    #[test]
    fn a_passenger_is_in_the_bus_ridden_in_not_by_the_parked_one() {
        let alex = pose(2, (662.0, 0.0));
        let buses = |id: u32| (id == 2).then(|| (DVec3::new(alex.x, alex.y, 0.0), 90.0));
        let mut michael = pose(3, (0.0, 0.0));
        michael.walker = Some(walker((661.0, 1.0), Some(2)));
        assert_eq!(player_at(&michael, buses).map(|p| p.0), Some(DVec3::new(662.0, 0.0, 0.0)));
        // on foot, out of any bus: where he walks
        michael.walker = Some(walker((120.0, 40.0), None));
        assert_eq!(player_at(&michael, buses).map(|p| p.0), Some(DVec3::new(120.0, 40.0, 0.0)));
        // aboard a bus that is gone: where he walks too
        michael.walker = Some(walker((5.0, 5.0), Some(9)));
        assert_eq!(player_at(&michael, buses).map(|p| p.0), Some(DVec3::new(5.0, 5.0, 0.0)));
        // driving: his bus
        michael.walker = None;
        assert_eq!(player_at(&michael, buses).map(|p| p.0), Some(DVec3::new(0.0, 0.0, 0.0)));
        // no bus and not on foot: nowhere
        let heartbeat = omsi_net::Pose { id: 4, ..Default::default() };
        assert_eq!(player_at(&heartbeat, buses), None);
    }
}
