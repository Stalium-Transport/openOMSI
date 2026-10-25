use omsi_net::Footprint;

const MARGIN: f64 = 1.5;

pub fn inside(f: &Footprint, local: [f32; 3]) -> bool {
    let (across, along) = (local[0] as f64, local[1] as f64);
    across.abs() <= f.width as f64 * 0.5 + MARGIN && along.abs() <= f.length as f64 + MARGIN
}

pub fn passenger_trigger(name: &str) -> bool {
    let n = name.trim().to_ascii_lowercase();
    let n = n.strip_suffix("_off").unwrap_or(&n);
    // (Stalium: the driver's switch that turns the stop requests off is not a passenger's)
    if n.contains("deaktiv") {
        return false;
    }
    n.starts_with("int_") || n.contains("haltewunsch") || n.contains("kinderwagenwunsch") || stalium_passenger_trigger(n)
}

/// Stalium: the passengers' buttons of other buses' makers, found in the Stalium fleet - a
/// door's request (`door_2_wunsch`, `door_3_wunsch_aussen`), the ramp's (`door_rampenwunsch`,
/// `ramplift_wunsch`, `RampRequestButtonPressed`), a stop request or bell by another name
/// (`door_stoprequest_1`, `StopBell`, `DisabledStopbell`). Never a cockpit's (`cp_...`).
fn stalium_passenger_trigger(n: &str) -> bool {
    !n.starts_with("cp_") && !n.starts_with("dash_") && (n.contains("wunsch") || n.contains("stoprequest") || n.contains("ramprequest") || n.contains("stopbell"))
}

/// May player `from` work switch `ev` of our bus `bus` (`lan`: our session)? Only a passenger
/// aboard our bus, inside it, and only a switch passengers work (`passenger_trigger`).
pub fn may_work(lan: &omsi_net::LanSession, from: u32, bus: Option<&omsi_sim::VehicleInstance>, ev: &str) -> bool {
    let walker = lan.peers().find(|p| p.pose.id == from).and_then(|p| p.pose.walker);
    let aboard = walker.and_then(|w| w.aboard).filter(|a| a.owner == lan.my_id);
    let inside = match (aboard, bus) {
        (Some(a), Some(v)) => inside(&crate::spawn::footprint_of(v, crate::spawn::BUS_BOX), a.local),
        _ => false,
    };
    aboard.is_some() && inside && passenger_trigger(ev)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_passenger_is_inside_the_bus_it_claims() {
        let bus = Footprint { x: 100.0, y: 50.0, z: 0.0, heading: 90.0, length: 12.0, width: 2.5 };
        assert!(inside(&bus, [0.0, 0.0, 1.0]));
        assert!(inside(&bus, [0.8, -5.5, 1.0]));
        assert!(!inside(&bus, [5.0, 0.0, 1.0]));
        assert!(!inside(&bus, [0.0, 200.0, 1.0]));
    }

    #[test]
    fn only_a_passengers_controls_are_worked_from_another_game() {
        assert!(passenger_trigger("int_haltewunsch"));
        assert!(passenger_trigger("Haltewunsch_off"));
        assert!(passenger_trigger("int_kinderwagenwunsch"));
        assert!(!passenger_trigger("bus_doorfront0"));
        assert!(!passenger_trigger("handbrake"));
        assert!(!passenger_trigger("engine_start"));
        // Stalium
        for n in ["door_2_wunsch", "door_3_wunsch_aussen", "door_rampenwunsch", "ramplift_wunsch", "RampRequestButtonPressed", "door_stoprequest_1", "StopBell", "DisabledStopbell", "door_behindiwunsch_off"] {
            assert!(passenger_trigger(n), "{n}");
        }
        for n in ["cp_Haltewunsch_deaktiviert", "cp_sprechwunsch", "cp_stop_request_toggle_a", "Ticketer_Ticket1Tapped", "cp_batterietrennschalter_toggle", "EngineStop", "INEO_Keypad_Valid"] {
            assert!(!passenger_trigger(n), "{n}");
        }
    }
}
