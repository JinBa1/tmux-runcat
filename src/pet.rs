use crate::animation::CatSize;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

const BASE: u32 = 0xf700;
const XLARGE_BASE: u32 = 0xe400;
const SPIN_BASE: u32 = 0x100000;
const MIRROR_BASE: u32 = 0x101000;
const WALL_BASE: u32 = 0x102000;
const WALL_MIRROR_BASE: u32 = 0x103000;
const REST_ACTIONS: [&str; 6] = ["lick-paw", "idle", "itch", "lick-body", "meow", "sit"];
const DEBUG_WALL_ACTIONS: [&str; 3] = ["head-bump", "wall-scratch", "parkour"];

#[derive(Clone, Copy)]
struct Clip {
    name: &'static str,
    first: u32,
    frames: u32,
}

fn clip(name: &str) -> Clip {
    static CLIPS: OnceLock<Vec<Clip>> = OnceLock::new();
    let clips = CLIPS.get_or_init(|| {
        let mut first = 0;
        include_str!("../fonts/pet-cats/animations.tsv")
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let fields: Vec<_> = line.split('\t').collect();
                let frames = fields[2].parse().expect("valid bundled Pet Cats manifest");
                let clip = Clip {
                    name: fields[0],
                    first,
                    frames,
                };
                first += frames;
                clip
            })
            .collect()
    });
    *clips
        .iter()
        .find(|clip| clip.name == name)
        .expect("known Pet Cats animation")
}

pub fn pose(size: CatSize, action: &str, frame: u32) -> String {
    let wall_first = clip("head-bump").first;
    let clip = clip(action);
    let frame = clip.first + frame % clip.frames;
    if action == "spin" {
        let (cells, offset) = size_layout(size);
        return (0..cells)
            .map(|half| {
                char::from_u32(SPIN_BASE + (frame - clip.first) * 10 + offset + half).unwrap()
            })
            .collect();
    }
    if matches!(action, "head-bump" | "wall-scratch" | "parkour") {
        let (cells, offset) = size_layout(size);
        return (0..cells)
            .map(|half| {
                char::from_u32(WALL_BASE + (frame - wall_first) * 10 + offset + half).unwrap()
            })
            .collect();
    }
    if size == CatSize::XLarge {
        return (0..4)
            .map(|half| char::from_u32(XLARGE_BASE + 4 * frame + half).unwrap())
            .collect();
    }
    let (cells, offset) = match size {
        CatSize::Small => (1, 0),
        CatSize::Medium => (2, 1),
        CatSize::Large => (3, 3),
        CatSize::XLarge => unreachable!(),
    };
    let first = BASE + 6 * frame + offset;
    (0..cells)
        .map(|half| char::from_u32(first + half).unwrap())
        .collect()
}

fn size_layout(size: CatSize) -> (u32, u32) {
    match size {
        CatSize::Small => (1, 0),
        CatSize::Medium => (2, 1),
        CatSize::Large => (3, 3),
        CatSize::XLarge => (4, 6),
    }
}

pub fn mirror(pose: &str) -> String {
    pose.chars()
        .map(|character| {
            let code = character as u32;
            let wall_first = clip("spin").first + clip("spin").frames;
            let wall_frames = clip("parkour").first + clip("parkour").frames - wall_first;
            let (frame, offset) = if (BASE..BASE + clip("spin").first * 6).contains(&code) {
                ((code - BASE) / 6, (code - BASE) % 6)
            } else if (XLARGE_BASE..XLARGE_BASE + clip("spin").first * 4).contains(&code) {
                ((code - XLARGE_BASE) / 4, 6 + (code - XLARGE_BASE) % 4)
            } else if (SPIN_BASE..SPIN_BASE + clip("spin").frames * 10).contains(&code) {
                (
                    clip("spin").first + (code - SPIN_BASE) / 10,
                    (code - SPIN_BASE) % 10,
                )
            } else if (WALL_BASE..WALL_BASE + wall_frames * 10).contains(&code) {
                (
                    wall_first + (code - WALL_BASE) / 10,
                    (code - WALL_BASE) % 10,
                )
            } else {
                return character;
            };
            if frame < wall_first {
                char::from_u32(MIRROR_BASE + frame * 10 + offset).unwrap()
            } else {
                char::from_u32(WALL_MIRROR_BASE + (frame - wall_first) * 10 + offset).unwrap()
            }
        })
        .collect()
}

pub struct Activity {
    pub now: f64,
    pub elapsed: f64,
    pub last_input: f64,
    pub pressed: bool,
    pub rate: f64,
    pub idle: f64,
    pub fps: f64,
    pub previous_fps: f64,
    pub maximum: Option<f64>,
    pub size: CatSize,
    pub spin_chance: f64,
}

pub struct Pet {
    pub action: &'static str,
    pub deadline: f64,
    started: f64,
    wake_since: Option<f64>,
    rest_cycle: usize,
    frame: u32,
    phase: f64,
    random: u64,
    next_spin_check: f64,
    spin_since: Option<f64>,
    wall_since: Option<f64>,
    wall_debug: bool,
    debug_index: usize,
    parkour_landed: Option<f64>,
}

impl Pet {
    pub fn new(now: f64) -> Self {
        Self {
            action: "sleep",
            deadline: now,
            started: now,
            wake_since: None,
            rest_cycle: 0,
            frame: 0,
            phase: 0.0,
            random: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(1, |time| time.as_nanos() as u64 | 1),
            next_spin_check: now + 1.0,
            spin_since: None,
            wall_since: None,
            wall_debug: false,
            debug_index: 0,
            parkour_landed: None,
        }
    }

    pub fn set_wall_debug(&mut self, enabled: bool) {
        if enabled && !self.wall_debug {
            self.debug_index = 0;
        }
        self.wall_debug = enabled;
    }

    pub fn hit_wall(
        &mut self,
        now: f64,
        chance: f64,
        debug: bool,
        size: CatSize,
    ) -> Option<String> {
        if !matches!(self.action, "walk" | "run") {
            return None;
        }
        if debug {
            self.action = DEBUG_WALL_ACTIONS[self.debug_index];
            self.debug_index = (self.debug_index + 1) % DEBUG_WALL_ACTIONS.len();
        } else {
            if chance <= 0.0 {
                return None;
            }
            self.random ^= self.random << 13;
            self.random ^= self.random >> 7;
            self.random ^= self.random << 17;
            if (self.random % 10_000) as f64 >= chance * 100.0 {
                return None;
            }
            let running = self.action == "run";
            self.random ^= self.random << 13;
            self.random ^= self.random >> 7;
            self.random ^= self.random << 17;
            if running {
                // Keep scratch's former one-in-five share after removing
                // cheek rub and paw tap.
                let choices = [
                    "head-bump",
                    "head-bump",
                    "wall-scratch",
                    "parkour",
                    "parkour",
                ];
                self.action = choices[(self.random % choices.len() as u64) as usize];
            } else if self.random % 3 == 0 {
                self.action = "wall-scratch";
            } else {
                return None;
            }
        }
        self.wall_since = Some(now);
        self.started = now;
        self.frame = 0;
        self.phase = 0.0;
        self.deadline = now + wall_duration(self.action) / clip(self.action).frames as f64;
        Some(pose(size, self.action, 0))
    }

    pub fn update(&mut self, activity: Activity) -> String {
        let Activity {
            now,
            elapsed,
            last_input,
            pressed,
            rate,
            idle,
            fps,
            previous_fps,
            maximum,
            size,
            spin_chance,
        } = activity;
        let pause = 0.35_f64.min(idle * 0.2);
        let lying = 0.8_f64.min(idle * 0.4);
        let waking = 0.65_f64.min(idle * 0.4);
        if pressed {
            self.parkour_landed = None;
        }
        if let Some(started) = self.wall_since
            && self.action == "parkour"
            && now - started >= wall_duration("parkour")
            && now - last_input >= idle - lying
        {
            // Finish the landing before starting a complete lie-down. The
            // idle clock used to skip directly from the jump to sleep.
            self.parkour_landed = Some(started + wall_duration("parkour"));
        }
        let last_input = self
            .parkour_landed
            .map_or(last_input, |landed| last_input.max(landed - idle + lying));
        let age = now - last_input;
        if pressed && matches!(self.action, "sleep" | "deep-sleep") {
            self.wake_since = Some(now);
        }

        let spinning = self.spin_since.is_some() && spin_chance > 0.0 && age < pause;
        if !spinning {
            self.spin_since = None;
        }
        let wall = self
            .wall_since
            .filter(|started| now - started < wall_duration(self.action));
        if wall.is_none() {
            self.wall_since = None;
        }
        let (mut action, mut started) = if let Some(started) = wall {
            (self.action, started)
        } else if let Some(landed) = self.parkour_landed.filter(|landed| now - landed < lying) {
            ("lie-down", landed)
        } else if age >= idle {
            self.spin_since = None;
            if last_input.is_finite() && age >= idle + 30.0 {
                ("deep-sleep", last_input + idle + 30.0)
            } else {
                (
                    "sleep",
                    if last_input.is_finite() {
                        last_input + idle
                    } else {
                        now
                    },
                )
            }
        } else if let Some(wake) = self.wake_since.filter(|wake| now - wake < waking) {
            ("stretch", wake)
        } else if spinning {
            ("spin", self.spin_since.unwrap())
        } else if age >= idle - lying {
            ("lie-down", last_input + idle - lying)
        } else if age >= pause {
            if !REST_ACTIONS.contains(&self.action) {
                self.rest_cycle = self.rest_cycle.wrapping_add(1);
            }
            (
                REST_ACTIONS[self.rest_cycle.wrapping_sub(1) % REST_ACTIONS.len()],
                last_input + pause,
            )
        } else if rate >= 8.0 || (matches!(self.action, "run" | "spin") && rate >= 6.0) {
            ("run", now)
        } else {
            ("walk", now)
        };

        if wall.is_none() && now >= self.next_spin_check {
            self.next_spin_check = now + 1.0;
            if action == "run" && spin_chance > 0.0 {
                self.random ^= self.random << 13;
                self.random ^= self.random >> 7;
                self.random ^= self.random << 17;
                if ((self.random % 10_000) as f64) < spin_chance * 100.0 {
                    self.spin_since = Some(now);
                    self.next_spin_check = now + 8.0;
                    action = "spin";
                    started = now;
                }
            }
        }

        let changed = action != self.action;
        if changed {
            self.action = action;
            self.started = started;
            self.frame = 0;
            self.phase = 0.0;
        }
        let clip = clip(action);
        let time = (now - self.started).max(0.0);
        if matches!(action, "walk" | "run" | "spin") {
            let spin_base = if action == "spin" {
                clip.frames as f64
            } else {
                0.0
            };
            let speed = maximum.map_or(fps + spin_base, |cap| (fps + spin_base).min(cap));
            let previous_speed = maximum.map_or(previous_fps + spin_base, |cap| {
                (previous_fps + spin_base).min(cap)
            });
            if !changed {
                let frames = self.phase + elapsed * (previous_speed + speed) * 0.5;
                self.frame = (self.frame + (frames % clip.frames as f64) as u32) % clip.frames;
                self.phase = frames.fract();
            }
            self.deadline = now + (1.0 - self.phase) / speed;
        } else if matches!(action, "sleep" | "deep-sleep") {
            let step = (time / 0.75).floor();
            self.frame = [0, 1, 2, 1][step as usize % 4];
            self.deadline = self.started + (step + 1.0) * 0.75;
        } else if matches!(action, "head-bump" | "wall-scratch" | "parkour") {
            if action == "parkour" {
                let (frame, end) = crate::parkour::sample(time);
                self.frame = frame;
                self.deadline = self.started + end;
            } else {
                let speed = clip.frames as f64 / wall_duration(action);
                let step = (time * speed).floor().min((clip.frames - 1) as f64);
                self.frame = step as u32;
                self.deadline = self.started + (step + 1.0) / speed;
            }
        } else {
            let (speed, once) = match action {
                "stretch" => (clip.frames as f64 / waking, true),
                "lie-down" => (clip.frames as f64 / lying, true),
                "meow" => (8.0, true),
                "itch" => (6.0, false),
                "sit" => (1.0, false),
                _ => (10.0, false),
            };
            let step = (time * speed).floor();
            self.frame = if once {
                (step as u32).min(clip.frames - 1)
            } else {
                (step % clip.frames as f64) as u32
            };
            self.deadline = self.started + (step + 1.0) / speed;
        }
        pose(size, action, self.frame)
    }
}

fn wall_duration(action: &str) -> f64 {
    match action {
        "head-bump" => 1.7,
        "wall-scratch" => 1.35,
        "parkour" => crate::parkour::duration(),
        _ => 1.45,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_clips_share_the_font_manifest_and_keep_the_selected_width() {
        let wall_frames = clip("parkour").first + clip("parkour").frames - clip("head-bump").first;
        for name in [
            "idle",
            "walk",
            "run",
            "meow",
            "lie-down",
            "itch",
            "sleep",
            "deep-sleep",
            "sit",
            "lick-paw",
            "lick-body",
            "stretch",
            "spin",
            "head-bump",
            "wall-scratch",
            "parkour",
        ] {
            let clip = clip(name);
            for frame in 0..clip.frames {
                for (size, width) in [
                    (CatSize::Small, 1),
                    (CatSize::Medium, 2),
                    (CatSize::Large, 3),
                    (CatSize::XLarge, 4),
                ] {
                    let text = pose(size, name, frame);
                    assert_eq!(text.chars().count(), width);
                    let range = if name == "spin" {
                        0x100000..0x1000a0
                    } else if matches!(name, "head-bump" | "wall-scratch" | "parkour") {
                        WALL_BASE..WALL_BASE + wall_frames * 10
                    } else if size == CatSize::XLarge {
                        0xe400..0xe518
                    } else {
                        0xf700..0xf8a4
                    };
                    assert!(text.chars().all(|value| range.contains(&(value as u32))));
                    let mirrored = mirror(&text);
                    assert_eq!(mirrored.chars().count(), width);
                    let mirrored_range = if matches!(name, "head-bump" | "wall-scratch" | "parkour")
                    {
                        WALL_MIRROR_BASE..WALL_MIRROR_BASE + wall_frames * 10
                    } else {
                        0x101000..0x10135c
                    };
                    assert!(
                        mirrored
                            .chars()
                            .all(|value| mirrored_range.contains(&(value as u32)))
                    );
                }
            }
        }
        assert_eq!(clip("stretch").first + clip("stretch").frames, 70);
        assert_eq!(clip("head-bump").first, 86);
        assert_eq!(
            crate::parkour::sample(wall_duration("parkour")).0 + 1,
            clip("parkour").frames
        );
    }

    #[test]
    fn stopping_during_parkour_finishes_landing_then_a_full_lie_down() {
        let mut pet = Pet::new(0.0);
        pet.action = "run";
        pet.debug_index = 2;
        pet.hit_wall(10.0, 100.0, true, CatSize::Large).unwrap();
        let mut update = |now| {
            pet.update(Activity {
                now,
                elapsed: 0.05,
                last_input: 10.0,
                pressed: false,
                rate: 0.0,
                idle: 2.0,
                fps: 3.0,
                previous_fps: 3.0,
                maximum: None,
                size: CatSize::Large,
                spin_chance: 0.0,
            })
        };
        let landing = update(12.39);
        assert_eq!(landing, pose(CatSize::Large, "parkour", 25));
        assert_eq!(update(12.4), pose(CatSize::Large, "lie-down", 0));
        assert_eq!(update(12.79), pose(CatSize::Large, "lie-down", 3));
        assert_eq!(update(13.21), pose(CatSize::Large, "sleep", 0));
    }

    #[test]
    fn wall_reactions_are_balanced_among_eligible_actions() {
        let walking_actions = ["wall-scratch"];
        let running_actions = ["head-bump", "wall-scratch", "parkour"];
        let mut walking_counts = [0; 1];
        let mut running_counts = [0; 3];
        for seed in 1..=1000 {
            let mut pet = Pet::new(0.0);
            pet.action = "walk";
            pet.random = seed;
            assert!(pet.hit_wall(1.0, 0.0, false, CatSize::Medium).is_none());
            if let Some(frame) = pet.hit_wall(1.0, 100.0, false, CatSize::Medium) {
                assert_eq!(frame.chars().count(), 2);
                walking_counts[walking_actions
                    .iter()
                    .position(|action| *action == pet.action)
                    .unwrap()] += 1;
            }

            let mut pet = Pet::new(0.0);
            pet.action = "run";
            pet.random = seed;
            assert!(pet.hit_wall(1.0, 100.0, false, CatSize::Medium).is_some());
            running_counts[running_actions
                .iter()
                .position(|action| *action == pet.action)
                .unwrap()] += 1;
        }
        assert!(
            walking_counts
                .iter()
                .all(|count| (270..390).contains(count))
        );
        assert!((350..450).contains(&running_counts[0]));
        assert!((150..250).contains(&running_counts[1]));
        assert!((350..450).contains(&running_counts[2]));
        assert_eq!(running_counts.iter().sum::<i32>(), 1000);
    }
}
