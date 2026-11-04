use crate::animation::{Settings, Skin};
use crate::pet;
use std::time::{SystemTime, UNIX_EPOCH};

const DEBUG_ACTIONS: [&str; 3] = ["head-bump", "wall-scratch", "parkour"];

/// A client's position and edge reaction, measured in terminal cells.
pub struct Patrol {
    position: f64,
    left: bool,
    updated: f64,
    action: Option<&'static str>,
    started: f64,
    wall_left: bool,
    launch_speed: f64,
    random: u64,
    debug_index: usize,
    was_debug: bool,
    landing: Option<f64>,
}

pub struct PatrolFrame {
    pub position: usize,
    pub left: bool,
    pub pose: Option<String>,
    pub state: Option<&'static str>,
}

impl Default for Patrol {
    fn default() -> Self {
        Self {
            position: 0.0,
            left: false,
            updated: 0.0,
            action: None,
            started: 0.0,
            wall_left: false,
            launch_speed: 0.0,
            random: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(1, |time| time.as_nanos() as u64 | 1),
            debug_index: 0,
            was_debug: false,
            landing: None,
        }
    }
}

impl Patrol {
    fn roll(&mut self) -> u64 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 7;
        self.random ^= self.random << 17;
        self.random
    }

    fn choose(&mut self, state: &str, settings: Settings) -> Option<&'static str> {
        if settings.wall_debug {
            let action = DEBUG_ACTIONS[self.debug_index];
            self.debug_index = (self.debug_index + 1) % DEBUG_ACTIONS.len();
            return Some(action);
        }
        if self.roll() % 10_000 >= (settings.wall_chance * 100.0) as u64 {
            return None;
        }
        if state == "run" {
            Some(
                [
                    "head-bump",
                    "head-bump",
                    "wall-scratch",
                    "parkour",
                    "parkour",
                ][(self.roll() % 5) as usize],
            )
        } else if self.roll().is_multiple_of(3) {
            Some("wall-scratch")
        } else {
            None
        }
    }

    pub fn update(
        &mut self,
        distance: f64,
        room: i32,
        now: f64,
        state: &str,
        settings: Settings,
        pressed: bool,
    ) -> PatrolFrame {
        if settings.wall_debug && !self.was_debug {
            self.debug_index = 0;
        }
        self.was_debug = settings.wall_debug;
        let elapsed = (now - self.updated).max(0.0);
        self.updated = now;
        let room = room.max(0) as f64;
        self.position = self.position.clamp(0.0, room);
        if pressed {
            self.landing = None;
        }
        if room == 0.0 || settings.skin != Skin::PetCats {
            self.action = None;
            self.landing = None;
        }

        if let Some(action) = self.action {
            let age = (now - self.started).max(0.0);
            let duration = duration(action);
            if age < duration {
                if action == "head-bump" {
                    let progress = ((age - 0.34) / 0.36).clamp(0.0, 1.0);
                    let recoil = 0.035 * room * progress * progress * (3.0 - 2.0 * progress);
                    self.position = if self.wall_left {
                        recoil
                    } else {
                        room - recoil
                    };
                } else if action == "parkour" {
                    let release = crate::parkour::phase_start("release");
                    if age >= release {
                        self.left = !self.wall_left;
                        let moving = (now - (now - elapsed).max(self.started + release)).max(0.0);
                        let landing = crate::parkour::phase_start("land");
                        let running = crate::parkour::phase_start("run");
                        let progress = ((age - landing) / (running - landing)).clamp(0.0, 1.0);
                        let blend = progress * progress * (3.0 - 2.0 * progress);
                        let current_speed = if elapsed > 0.0 {
                            (distance / elapsed).clamp(room * 0.08, room * 0.3)
                        } else {
                            0.0
                        };
                        let speed = self.launch_speed + (current_speed - self.launch_speed) * blend;
                        self.position += if self.left {
                            -moving * speed
                        } else {
                            moving * speed
                        };
                        self.position = self.position.clamp(0.0, room);
                    }
                }
                let frame = if action == "parkour" {
                    crate::parkour::sample(age).0
                } else {
                    let frames = if action == "head-bump" { 20 } else { 16 };
                    ((age * frames as f64 / duration).floor() as u32).min(frames - 1)
                };
                let pose = pet::pose(settings.size, action, frame);
                return PatrolFrame {
                    position: self.position.round() as usize,
                    left: self.left,
                    pose: Some(if self.wall_left {
                        pet::mirror(&pose)
                    } else {
                        pose
                    }),
                    state: Some(action),
                };
            }
            self.action = None;
            if action != "parkour" {
                self.left = !self.wall_left;
            } else if !settings.wall_debug && !matches!(state, "walk" | "run" | "spin") {
                self.landing = Some(now);
            }
        }

        if let Some(landed) = self.landing {
            let age = now - landed;
            if age < 0.6 && !pressed {
                let frame = ((age / 0.6 * 8.0) as u32).min(7);
                let pose = pet::pose(settings.size, "lie-down", frame);
                return PatrolFrame {
                    position: self.position.round() as usize,
                    left: self.left,
                    pose: Some(if self.left { pet::mirror(&pose) } else { pose }),
                    state: Some("lie-down"),
                };
            }
            self.landing = None;
        }

        if room > 0.0 && matches!(state, "walk" | "run") && distance.is_finite() && distance > 0.0 {
            let next = self.position + if self.left { -distance } else { distance };
            if next <= 0.0 || next >= room {
                let previous = self.position;
                self.position = if self.left { 0.0 } else { room };
                let wall_left = self.left;
                if settings.skin == Skin::PetCats
                    && let Some(action) = self.choose(state, settings)
                {
                    self.action = Some(action);
                    self.started = now;
                    self.wall_left = wall_left;
                    self.launch_speed = if elapsed > 0.0 {
                        (distance / elapsed).clamp(room * 0.08, room * 0.3)
                    } else {
                        0.0
                    };
                    let pose = pet::pose(settings.size, action, 0);
                    return PatrolFrame {
                        position: self.position.round() as usize,
                        left: self.left,
                        pose: Some(if wall_left { pet::mirror(&pose) } else { pose }),
                        state: Some(action),
                    };
                }
                // Keep upstream's fractional overshoot when no reaction plays.
                let start = if wall_left {
                    2.0 * room - previous
                } else {
                    previous
                };
                let phase = (start + distance).rem_euclid(2.0 * room);
                self.left = phase >= room;
                self.position = if self.left { 2.0 * room - phase } else { phase };
            } else {
                self.position = next;
            }
        }
        PatrolFrame {
            position: self.position.round() as usize,
            left: self.left,
            pose: None,
            state: None,
        }
    }
}

fn duration(action: &str) -> f64 {
    match action {
        "head-bump" => 1.7,
        "wall-scratch" => 1.35,
        "parkour" => crate::parkour::duration(),
        _ => unreachable!("unknown wall action"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::CatSize;

    fn debug() -> Settings {
        Settings {
            skin: Skin::PetCats,
            size: CatSize::Large,
            wall_debug: true,
            ..Settings::default()
        }
    }

    #[test]
    fn each_client_reacts_at_its_own_edge() {
        let (mut narrow, mut wide) = (Patrol::default(), Patrol::default());
        let settings = debug();
        for step in 1..4 {
            let now = step as f64 * 0.05;
            let first = narrow.update(1.0, 4, now, "run", settings, true);
            let second = wide.update(1.0, 10, now, "run", settings, true);
            assert_eq!(first.position, second.position);
            assert_eq!(first.state, None);
            assert_eq!(second.state, None);
        }
        let first = narrow.update(1.0, 4, 0.2, "run", settings, true);
        let second = wide.update(1.0, 10, 0.2, "run", settings, true);
        assert_eq!((first.position, first.state), (4, Some("head-bump")));
        assert_eq!((second.position, second.state), (4, None));
        assert!(
            first
                .pose
                .unwrap()
                .chars()
                .all(|glyph| { (0x102000..0x10226c).contains(&(glyph as u32)) })
        );
    }

    #[test]
    fn ordinary_turns_preserve_stride_overshoot() {
        let mut patrol = Patrol::default();
        let settings = Settings {
            wall_chance: 0.0,
            ..Settings::default()
        };
        assert_eq!(
            patrol.update(7.0, 10, 0.1, "run", settings, true).position,
            7
        );
        let turned = patrol.update(4.0, 10, 0.2, "run", settings, true);
        assert_eq!((turned.position, turned.left), (9, true));
        let turned_again = patrol.update(9.4, 10, 0.3, "run", settings, true);
        assert_eq!((turned_again.position, turned_again.left), (0, false));
    }

    #[test]
    fn debug_cycle_and_parkour_rebound() {
        let mut patrol = Patrol::default();
        let settings = debug();
        let first = patrol.update(4.0, 4, 0.1, "run", settings, true);
        assert_eq!(first.state, Some("head-bump"));
        let recoil = patrol.update(0.0, 4, 0.9, "run", settings, false);
        assert!(patrol.position < 4.0);
        assert_eq!(recoil.state, Some("head-bump"));
        patrol.update(0.0, 4, 1.81, "run", settings, false);
        let second = patrol.update(4.0, 4, 1.86, "run", settings, true);
        assert_eq!(second.state, Some("wall-scratch"));
        assert!(second.left);
        patrol.update(0.0, 4, 3.22, "run", settings, false);
        let third = patrol.update(4.0, 4, 3.27, "run", settings, true);
        assert_eq!(third.state, Some("parkour"));
        assert!(!third.left);
        let contact = patrol.update(0.0, 4, 4.7, "run", settings, false);
        assert_eq!((contact.position, contact.left), (4, false));
        let launch = patrol.update(0.0, 4, 5.15, "run", settings, false);
        assert!(launch.left && patrol.position < 4.0);
        assert!(
            launch
                .pose
                .unwrap()
                .chars()
                .all(|glyph| { (0x102000..0x10226c).contains(&(glyph as u32)) })
        );
    }

    #[test]
    fn running_choices_keep_scratch_rare_and_walking_cannot_bump() {
        let settings = Settings {
            skin: Skin::PetCats,
            wall_chance: 100.0,
            ..Settings::default()
        };
        let mut counts = [0; 3];
        for seed in 1..=1000 {
            let mut patrol = Patrol {
                random: seed,
                ..Patrol::default()
            };
            let action = patrol.choose("run", settings).unwrap();
            counts[DEBUG_ACTIONS
                .iter()
                .position(|choice| *choice == action)
                .unwrap()] += 1;
            let walking = patrol.choose("walk", settings);
            assert!(walking.is_none() || walking == Some("wall-scratch"));
        }
        assert!((350..450).contains(&counts[0]));
        assert!((150..250).contains(&counts[1]));
        assert!((350..450).contains(&counts[2]));
    }

    #[test]
    fn parkour_finishes_then_shows_a_full_lie_down_when_typing_stops() {
        let mut patrol = Patrol {
            position: 4.0,
            action: Some("parkour"),
            started: 10.0,
            updated: 12.35,
            ..Patrol::default()
        };
        let settings = debug();
        let landing = patrol.update(0.0, 4, 12.39, "sleep", settings, false);
        assert_eq!(landing.state, Some("parkour"));
        let mut normal = settings;
        normal.wall_debug = false;
        let lie_down = patrol.update(0.0, 4, 12.41, "sleep", normal, false);
        assert_eq!(lie_down.state, Some("lie-down"));
        assert_eq!(
            lie_down.pose.unwrap(),
            pet::mirror(&pet::pose(CatSize::Large, "lie-down", 0))
        );
        assert_eq!(
            patrol.update(0.0, 4, 12.8, "sleep", normal, false).state,
            Some("lie-down")
        );
        assert_eq!(
            patrol.update(0.0, 4, 13.02, "sleep", normal, false).state,
            None
        );
    }
}
