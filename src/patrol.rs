pub struct Patrol {
    phase: f64,
    updated: f64,
}

impl Patrol {
    pub fn new(now: f64) -> Self {
        Self {
            phase: 0.0,
            updated: now,
        }
    }

    pub fn update(&mut self, now: f64, state: &str, fps: f64) -> (u16, bool) {
        let elapsed = (now - self.updated).max(0.0);
        self.updated = now;
        if matches!(state, "walk" | "run") {
            let next = self.phase + elapsed * (fps * 0.0125).clamp(0.08, 0.3);
            // Publish the endpoint itself for one frame before returning.
            self.phase = if self.phase < 1.0 && next >= 1.0 {
                1.0
            } else if next >= 2.0 {
                0.0
            } else {
                next
            };
        }
        let left = self.phase >= 1.0;
        let position = if left { 2.0 - self.phase } else { self.phase };
        ((position * 1000.0).round() as u16, left)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounces_and_stops_for_spinning_and_rest() {
        let mut patrol = Patrol::new(0.0);
        assert_eq!(patrol.update(4.0, "run", 20.0), (1000, true));
        assert_eq!(patrol.update(5.0, "run", 20.0), (750, true));
        assert_eq!(patrol.update(6.0, "spin", 20.0), (750, true));
        assert_eq!(patrol.update(7.0, "sleep", 20.0), (750, true));
        assert_eq!(patrol.update(10.0, "run", 20.0), (0, false));
    }

    #[test]
    fn long_delays_and_clock_reversal_stay_in_bounds() {
        let mut patrol = Patrol::new(0.0);
        for now in [100.0, 99.0, 500.0, 500.01] {
            assert!(patrol.update(now, "run", 300.0).0 <= 1000);
        }
    }
}
