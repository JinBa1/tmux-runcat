/// A client's position in terminal cells. Retain fractional travel between
/// redraws; neither terminal width nor polling frequency changes stride length.
#[derive(Default)]
pub struct Patrol {
    position: f64,
    left: bool,
}

impl Patrol {
    pub fn update(&mut self, distance: f64, room: i32) -> (usize, bool) {
        let room = room.max(0) as f64;
        self.position = self.position.clamp(0.0, room);
        if room > 0.0 && distance.is_finite() && distance > 0.0 {
            let phase = if self.left {
                2.0 * room - self.position
            } else {
                self.position
            };
            let phase = (phase + distance).rem_euclid(2.0 * room);
            self.left = phase >= room;
            self.position = if self.left { 2.0 * room - phase } else { phase };
        }
        (self.position.round() as usize, self.left)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_fractional_travel_and_reflects_overshoot() {
        let mut patrol = Patrol::default();
        for _ in 0..4 {
            patrol.update(0.1, 10);
        }
        assert_eq!(patrol.update(0.2, 10), (1, false));
        assert_eq!(patrol.update(10.0, 10), (9, true));
        assert_eq!(patrol.update(9.4, 10), (0, false));
        assert_eq!(patrol.update(1003.0, 10), (3, false));
    }

    #[test]
    fn width_does_not_change_speed_and_resizing_clamps_position() {
        let (mut narrow, mut wide) = (Patrol::default(), Patrol::default());
        assert_eq!(narrow.update(7.0, 10), wide.update(7.0, 100));
        assert_eq!(wide.update(0.0, 4), (4, false));
        assert_eq!(wide.update(1.0, 4), (3, true));
        assert_eq!(wide.update(0.0, 100), (3, true));
        assert_eq!(wide.update(0.0, -10), (0, true));
    }
}
