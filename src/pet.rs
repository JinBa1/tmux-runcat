use crate::animation::CatSize;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

const BASE: u32 = 0xf700;
const XLARGE_BASE: u32 = 0xe400;
const SPIN_BASE: u32 = 0x100000;
const MIRROR_BASE: u32 = 0x101000;
const REST_ACTIONS: [&str; 6] = ["lick-paw", "idle", "itch", "lick-body", "meow", "sit"];

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
            let (frame, offset) = if (BASE..BASE + clip("spin").first * 6).contains(&code) {
                ((code - BASE) / 6, (code - BASE) % 6)
            } else if (XLARGE_BASE..XLARGE_BASE + clip("spin").first * 4).contains(&code) {
                ((code - XLARGE_BASE) / 4, 6 + (code - XLARGE_BASE) % 4)
            } else if (SPIN_BASE..SPIN_BASE + clip("spin").frames * 10).contains(&code) {
                (
                    clip("spin").first + (code - SPIN_BASE) / 10,
                    (code - SPIN_BASE) % 10,
                )
            } else {
                return character;
            };
            char::from_u32(MIRROR_BASE + frame * 10 + offset).unwrap()
        })
        .collect()
}

pub struct Activity {
    pub now: f64,
    pub elapsed: f64,
    pub last_input: f64,
    pub pressed: bool,
    pub idle: f64,
    pub fps: f64,
    pub minimum: f64,
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
    rest_input: f64,
    sleep_limit: f64,
    fps: f64,
    pub travel: f64,
    frame: u32,
    phase: f64,
    random: u64,
    next_spin_check: f64,
    spin_since: Option<f64>,
}

impl Pet {
    pub fn new(now: f64) -> Self {
        Self {
            action: "sleep",
            deadline: now,
            started: now,
            wake_since: None,
            rest_cycle: 0,
            rest_input: f64::NEG_INFINITY,
            sleep_limit: f64::INFINITY,
            fps: 3.0,
            travel: 0.0,
            frame: 0,
            phase: 0.0,
            random: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(1, |time| time.as_nanos() as u64 | 1),
            next_spin_check: now + 1.0,
            spin_since: None,
        }
    }

    pub fn update(&mut self, activity: Activity) -> String {
        let Activity {
            now,
            elapsed,
            last_input,
            pressed,
            idle,
            fps,
            minimum,
            maximum,
            size,
            spin_chance,
        } = activity;
        self.travel = 0.0;
        let age = now - last_input;
        let pause = 2.0_f64.min(idle * 0.65);
        let spin_pause = 0.35_f64.min(idle * 0.2);
        let lying = 0.6_f64.min(idle * 0.3);
        if pressed {
            self.sleep_limit = f64::INFINITY;
        }
        // Select once per pause, before computing this gesture's deadline.
        if age >= pause && last_input.is_finite() && self.rest_input != last_input {
            self.rest_cycle = self.rest_cycle.wrapping_add(1);
            self.rest_input = last_input;
        }
        let gesture = REST_ACTIONS[self.rest_cycle.wrapping_sub(1) % REST_ACTIONS.len()];
        let duration = match gesture {
            "lick-paw" | "lick-body" | "itch" => 3.0,
            "meow" => 1.0,
            _ => 0.5,
        };
        let idle = idle.min(pause + duration + lying).min(self.sleep_limit);
        if age >= idle {
            self.sleep_limit = idle;
        }
        let waking = 0.65_f64.min(idle * 0.4);
        // Ease to minimum cadence early enough to show a slow walk. The
        // same cadence advances both feet and travel, including on resume.
        let remaining = (1.0 - age / (pause * 0.7)).clamp(0.0, 1.0);
        let easing = remaining * remaining * (3.0 - 2.0 * remaining);
        let previous_fps = self.fps;
        let target = minimum + (fps - minimum) * easing * easing;
        self.fps = if target > previous_fps {
            previous_fps + (target - previous_fps) * (1.0 - 0.15_f64.powf(elapsed / 0.3))
        } else {
            target
        };
        if let Some(cap) = maximum {
            self.fps = self.fps.min(cap);
        }
        let fps = self.fps;
        if pressed && matches!(self.action, "sleep" | "deep-sleep") {
            self.wake_since = Some(now);
        }

        let spinning = self.spin_since.is_some() && spin_chance > 0.0 && age < spin_pause;
        if !spinning {
            self.spin_since = None;
        }
        let run_at = (minimum + 9.0)
            .min(maximum.unwrap_or(f64::INFINITY) * 0.85)
            .max(minimum);
        let walk_at = (minimum + 6.0).min(run_at - 0.5).max(minimum);
        let (mut action, mut started) = if age >= idle {
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
            (gesture, last_input + pause)
        } else if fps >= run_at || (matches!(self.action, "run" | "spin") && fps >= walk_at) {
            ("run", now)
        } else {
            ("walk", now)
        };

        if now >= self.next_spin_check {
            self.next_spin_check = now + 1.0;
            if action == "run" && spin_chance > 0.0 && age < spin_pause {
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

        let changing_gait =
            matches!(action, "walk" | "run") && matches!(self.action, "walk" | "run");
        let changed = action != self.action;
        if changed {
            self.action = action;
            self.started = started;
            if !changing_gait {
                self.frame = 0;
                self.phase = 0.0;
            }
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
            if !changed || changing_gait {
                let advance = elapsed * (previous_speed + speed) * 0.5;
                if action != "spin" {
                    let widths = if action == "run" { 1.5 } else { 1.0 };
                    self.travel =
                        advance / clip.frames as f64 * size_layout(size).0 as f64 * widths;
                }
                let frames = self.phase + advance;
                self.frame = (self.frame + (frames % clip.frames as f64) as u32) % clip.frames;
                self.phase = frames.fract();
            }
            self.deadline = now + (1.0 - self.phase) / speed;
        } else if matches!(action, "sleep" | "deep-sleep") {
            let step = (time / 0.75).floor();
            self.frame = [0, 1, 2, 1][step as usize % 4];
            self.deadline = self.started + (step + 1.0) * 0.75;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn activity(now: f64, last_input: f64, fps: f64) -> Activity {
        Activity {
            now,
            elapsed: 0.05,
            last_input,
            pressed: now == last_input,
            idle: 8.0,
            fps,
            minimum: 3.0,
            maximum: None,
            size: CatSize::XLarge,
            spin_chance: 0.0,
        }
    }

    #[test]
    fn each_gesture_finishes_at_its_own_deadline() {
        for (index, duration) in [3.0, 0.5, 3.0, 3.0, 1.0, 0.5].into_iter().enumerate() {
            let mut pet = Pet {
                action: "walk",
                rest_cycle: index,
                ..Pet::new(0.0)
            };
            pet.update(activity(2.01, 0.0, 3.0));
            assert_eq!(pet.action, REST_ACTIONS[index]);
            pet.update(activity(2.0 + duration - 0.01, 0.0, 3.0));
            assert_eq!(pet.action, REST_ACTIONS[index]);
            pet.update(activity(2.0 + duration + 0.01, 0.0, 3.0));
            assert_eq!(pet.action, "lie-down");
            pet.update(activity(2.6 + duration + 0.01, 0.0, 3.0));
            assert_eq!(pet.action, "sleep");
            let mut longer = activity(2.6 + duration + 0.02, 0.0, 3.0);
            longer.idle = 3600.0;
            pet.update(longer);
            assert_eq!(pet.action, "sleep", "changing the timeout woke the cat");
            pet.update(activity(32.6 + duration + 0.01, 0.0, 3.0));
            assert_eq!(pet.action, "deep-sleep");
            pet.update(activity(40.0, 40.0, 3.0));
            assert_eq!(pet.action, "stretch");
        }
    }

    #[test]
    fn gait_changes_preserve_footfall_and_fractional_progress() {
        let mut pet = Pet {
            action: "walk",
            frame: 5,
            phase: 0.4,
            fps: 11.0,
            ..Pet::new(0.0)
        };
        for (now, fps, action) in [(0.01, 63.0, "run"), (0.02, 3.0, "walk")] {
            let previous = pet.phase;
            let mut input = activity(now, now, fps);
            input.elapsed = 0.01;
            pet.update(input);
            assert_eq!(pet.action, action);
            assert_eq!(pet.frame, 5);
            assert!(pet.phase > previous);
            assert!(pet.deadline > now);
        }
    }

    #[test]
    fn travel_and_legs_use_the_same_cadence_and_stride() {
        for (action, fps, widths) in [("walk", 6.0, 1.0), ("run", 16.0, 1.5)] {
            let mut pet = Pet {
                action,
                fps,
                frame: 2,
                ..Pet::new(0.0)
            };
            let mut input = activity(0.1, 0.1, fps);
            input.elapsed = 0.1;
            pet.update(input);
            let frames = 0.1 * fps;
            assert!((pet.travel - frames / 8.0 * 4.0 * widths).abs() < 1e-9);
            assert_eq!(pet.frame, 2 + frames as u32);
            assert!((pet.phase - frames.fract()).abs() < 1e-9);
        }
    }

    #[test]
    fn pauses_ease_into_a_visible_walk_and_resume_without_jumping() {
        let mut pet = Pet {
            action: "run",
            fps: 63.0,
            ..Pet::new(0.0)
        };
        let mut walking = 0;
        let mut poses = std::collections::HashSet::new();
        for step in 1..40 {
            let previous = pet.fps;
            let pose = pet.update(activity(step as f64 * 0.05, 0.0, 63.0));
            assert!(pet.fps <= previous);
            assert!(pet.travel > 0.0);
            if pet.action == "walk" {
                walking += 1;
                poses.insert(pose);
            }
        }
        assert!(walking >= 20);
        assert!(poses.len() >= 4);
        let mut resumed = activity(1.96, 1.96, 63.0);
        resumed.elapsed = 0.01;
        pet.update(resumed);
        assert_eq!(pet.action, "walk");
        assert!(pet.fps < 20.0);
    }

    #[test]
    fn small_timeouts_compress_phases_and_rest_never_moves() {
        let mut pet = Pet::new(0.0);
        for step in 0..=101 {
            let now = step as f64 * 0.001;
            let mut input = activity(now, 0.0, 20.0);
            input.elapsed = 0.001;
            input.idle = 0.1;
            pet.update(input);
            if !matches!(pet.action, "walk" | "run") {
                assert_eq!(pet.travel, 0.0);
            }
        }
        assert_eq!(pet.action, "sleep");
    }

    #[test]
    fn all_clips_share_the_font_manifest_and_keep_the_selected_width() {
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
                    } else if size == CatSize::XLarge {
                        0xe400..0xe518
                    } else {
                        0xf700..0xf8a4
                    };
                    assert!(text.chars().all(|value| range.contains(&(value as u32))));
                    let mirrored = mirror(&text);
                    assert_eq!(mirrored.chars().count(), width);
                    assert!(
                        mirrored
                            .chars()
                            .all(|value| (0x101000..0x10135c).contains(&(value as u32)))
                    );
                }
            }
        }
        assert_eq!(clip("stretch").first + clip("stretch").frames, 70);
    }
}
