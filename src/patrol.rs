pub struct Patrol {
    position: f64,
    updated: f64,
    left: bool,
    turn_pending: bool,
    wall_since: Option<f64>,
    wall_left: bool,
    wall_speed: f64,
}

pub struct PatrolFrame {
    pub position: u16,
    pub left: bool,
    pub arrived: bool,
    pub mirror: bool,
}

impl Patrol {
    pub fn new(now: f64) -> Self {
        Self {
            position: 0.0,
            updated: now,
            left: false,
            turn_pending: false,
            wall_since: None,
            wall_left: false,
            wall_speed: 0.0,
        }
    }

    pub fn wall_event(&mut self, now: f64, fps: f64) {
        self.wall_since = Some(now);
        self.wall_left = self.left;
        self.wall_speed = (fps * 0.0125).clamp(0.08, 0.3);
    }

    pub fn update(&mut self, now: f64, state: &str, fps: f64) -> PatrolFrame {
        let elapsed = (now - self.updated).max(0.0);
        self.updated = now;
        let mut arrived = false;
        if state == "parkour" {
            if let Some(started) = self.wall_since {
                let age = now - started;
                let release = crate::parkour::phase_start("release");
                if age >= release && self.turn_pending {
                    self.left = !self.left;
                    self.turn_pending = false;
                }
                // All paw contacts share one world position. Travel begins
                // only after the rear paws release, then continues through
                // landing into ordinary locomotion, without an end stop.
                let moving = (now - (now - elapsed).max(started + release)).max(0.0);
                let landing = crate::parkour::phase_start("land");
                let running = crate::parkour::phase_start("run");
                let progress = ((age - landing) / (running - landing)).clamp(0.0, 1.0);
                let blend = progress * progress * (3.0 - 2.0 * progress);
                let speed =
                    self.wall_speed + ((fps * 0.0125).clamp(0.08, 0.3) - self.wall_speed) * blend;
                self.position = (self.position
                    + if self.left {
                        -moving * speed
                    } else {
                        moving * speed
                    })
                .clamp(0.0, 1.0);
            }
        } else if state == "head-bump" {
            if let Some(started) = self.wall_since {
                // Take a small step away after impact, then sit at that spot.
                let progress = ((now - started - 0.34) / 0.36).clamp(0.0, 1.0);
                let recoil = 0.035 * progress * progress * (3.0 - 2.0 * progress);
                self.position = if self.left { recoil } else { 1.0 - recoil };
            }
        } else if matches!(state, "walk" | "run") {
            if self.turn_pending {
                self.left = !self.left;
                self.turn_pending = false;
                self.wall_since = None;
            }
            let distance = elapsed * (fps * 0.0125).clamp(0.08, 0.3);
            let next = self.position + if self.left { -distance } else { distance };
            if next <= 0.0 || next >= 1.0 {
                self.position = if self.left { 0.0 } else { 1.0 };
                self.turn_pending = true;
                arrived = true;
            } else {
                self.position = next;
            }
        }
        PatrolFrame {
            position: (self.position * 1000.0).round() as u16,
            left: self.left,
            arrived,
            // Parkour artwork contains its turn. Reflect the whole clip for
            // the left wall, independently of the outgoing travel direction.
            mirror: if state == "parkour" && self.wall_since.is_some() {
                self.wall_left
            } else {
                self.left
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reaches_both_ends_and_holds_through_reaction() {
        let mut patrol = Patrol::new(0.0);
        let at_right = patrol.update(4.0, "run", 20.0);
        assert_eq!(
            (at_right.position, at_right.left, at_right.arrived),
            (1000, false, true)
        );
        patrol.wall_event(4.0, 20.0);
        let held = patrol.update(4.25, "head-bump", 20.0);
        assert_eq!((held.position, held.left), (1000, false));
        let recoiled = patrol.update(4.8, "head-bump", 20.0);
        assert_eq!((recoiled.position, recoiled.left), (965, false));
        let return_trip = patrol.update(5.0, "run", 20.0);
        assert!(return_trip.left && return_trip.position < 965);
        assert_eq!(
            patrol.update(6.0, "spin", 20.0).position,
            return_trip.position
        );
        let left_end = patrol.update(10.0, "run", 20.0);
        assert_eq!(
            (left_end.position, left_end.left, left_end.arrived),
            (0, true, true)
        );
    }

    #[test]
    fn long_delays_and_clock_reversal_stay_in_bounds() {
        let mut patrol = Patrol::new(0.0);
        for now in [100.0, 99.0, 500.0, 500.01] {
            assert!(patrol.update(now, "run", 300.0).position <= 1000);
        }
    }

    #[test]
    fn parkour_turns_in_air_and_rebounds_from_both_ends() {
        let mut patrol = Patrol::new(0.0);
        assert!(patrol.update(4.0, "run", 20.0).arrived);
        patrol.wall_event(4.0, 20.0);
        assert_eq!(patrol.update(4.4, "parkour", 20.0).position, 1000);
        assert_eq!(patrol.update(4.9, "parkour", 20.0).position, 1000);
        assert!(!patrol.update(5.05, "parkour", 20.0).left);
        assert_eq!(patrol.update(5.6, "parkour", 20.0).position, 1000);
        let launched = patrol.update(5.8, "parkour", 20.0);
        assert!(launched.left && launched.position < 1000);
        assert!(
            !launched.mirror,
            "turning must not reflect the planted artwork"
        );
        let landed = patrol.update(6.3, "parkour", 20.0);
        assert_eq!((landed.position, landed.left), (838, true));
        let moving = patrol.update(6.5, "run", 20.0);
        assert!(moving.left && moving.position < landed.position);
        assert!(moving.mirror);
        assert!(patrol.update(10.5, "run", 20.0).arrived);
        patrol.wall_event(10.5, 20.0);
        assert!(patrol.update(11.5, "parkour", 20.0).left);
        let outgoing = patrol.update(12.3, "parkour", 20.0);
        assert!(!outgoing.left && outgoing.mirror);
        let landing = patrol.update(12.8, "parkour", 20.0);
        assert!(landing.position > outgoing.position);
    }
}
