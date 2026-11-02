use crate::patrol::Patrol;
use crate::pet::{Activity, Pet};
use std::collections::VecDeque;

pub const SLEEP_POSE: &str = "\u{effa}\u{effb}";
#[cfg(test)]
const SLEEP_POSES: [&str; 3] = [SLEEP_POSE, "\u{effc}\u{effd}", "\u{effe}\u{efff}"];
const SLEEP_STEP_SECONDS: f64 = 0.75;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Skin {
    #[default]
    Classic,
    PetCats,
}

impl Skin {
    fn parse(value: &str) -> Self {
        if value.trim().eq_ignore_ascii_case("pet-cats") {
            Self::PetCats
        } else {
            Self::Classic
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CatSize {
    Small,
    #[default]
    Medium,
    Large,
    XLarge,
}

impl CatSize {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "small" | "1" => Self::Small,
            "large" | "3" => Self::Large,
            "xlarge" | "4" => Self::XLarge,
            _ => Self::Medium,
        }
    }

    fn pose(self, frame: u32, sleeping: bool) -> String {
        let index = if sleeping { 5 + frame } else { frame };
        let points: Vec<u32> = match self {
            Self::Small => vec![0xf600 + index],
            Self::Medium if sleeping => vec![0xeffa + 2 * frame, 0xeffb + 2 * frame],
            Self::Medium => vec![0xeff0 + frame, 0xeff5 + frame],
            Self::Large => (0..3).map(|half| 0xf620 + 3 * index + half).collect(),
            Self::XLarge => (0..4).map(|half| 0xf640 + 4 * index + half).collect(),
        };
        points
            .into_iter()
            .map(|point| char::from_u32(point).unwrap())
            .collect()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub idle: f64,
    pub minimum: f64,
    pub maximum: Option<f64>,
    pub size: CatSize,
    pub skin: Skin,
    pub spin_chance: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            idle: 2.0,
            minimum: 3.0,
            maximum: None,
            size: CatSize::Medium,
            skin: Skin::Classic,
            spin_chance: 0.0,
        }
    }
}

impl Settings {
    pub fn parse(idle: &str, minimum: &str, maximum: &str, size: &str, skin: &str) -> Self {
        fn number(value: &str, fallback: f64, low: f64, high: f64) -> f64 {
            value
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .unwrap_or(fallback)
                .clamp(low, high)
        }
        let minimum = number(minimum, 3.0, 1.0, 20.0);
        Self {
            idle: number(idle, 2.0, 0.1, 3600.0),
            minimum,
            maximum: maximum
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite() && *value > 0.0)
                .map(|value| value.max(minimum)),
            size: CatSize::parse(size),
            skin: Skin::parse(skin),
            spin_chance: 0.0,
        }
    }

    pub fn with_spin_chance(mut self, value: &str) -> Self {
        self.spin_chance = value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
            .clamp(0.0, 100.0);
        self
    }
}

pub struct Animation {
    count: u64,
    last_input: f64,
    samples: VecDeque<(f64, u64)>,
    fps: f64,
    frame: u32,
    phase: f64,
    updated: f64,
    pose: String,
    sleep_since: Option<f64>,
    sleep_step: usize,
    skin: Skin,
    state: &'static str,
    pet: Pet,
    patrol: Patrol,
}

impl Animation {
    pub fn new(count: u64, now: f64) -> Self {
        Self {
            count,
            last_input: f64::NEG_INFINITY,
            samples: VecDeque::new(),
            fps: 3.0,
            frame: 4,
            phase: 0.0,
            updated: now,
            pose: SLEEP_POSE.to_owned(),
            sleep_since: Some(now),
            sleep_step: 0,
            skin: Skin::Classic,
            state: "sleep",
            pet: Pet::new(now),
            patrol: Patrol::new(now),
        }
    }

    pub fn update(&mut self, count: u64, now: f64, settings: Settings) -> &str {
        let elapsed = (now - self.updated).max(0.0);
        self.updated = now;
        let delta = count.saturating_sub(self.count);
        self.count = count;
        if delta > 0 {
            self.last_input = now;
            self.samples.push_back((now, delta));
        }
        while self.samples.front().is_some_and(|(at, _)| now - at >= 1.0) {
            self.samples.pop_front();
        }
        self.skin = settings.skin;
        let rate: f64 = self.samples.iter().map(|(_, count)| *count as f64).sum();
        if settings.skin == Skin::PetCats {
            let previous_fps = self.fps;
            if now - self.last_input >= settings.idle {
                self.samples.clear();
                self.fps = settings.minimum;
            } else {
                self.adjust_speed(rate, elapsed, settings);
            }
            self.pose = self.pet.update(Activity {
                now,
                elapsed,
                last_input: self.last_input,
                pressed: delta > 0,
                rate,
                idle: settings.idle,
                fps: self.fps,
                previous_fps,
                size: settings.size,
                spin_chance: settings.spin_chance,
            });
            self.state = self.pet.action;
            return &self.pose;
        }
        if now - self.last_input >= settings.idle {
            self.samples.clear();
            self.fps = settings.minimum;
            self.phase = 0.0;
            let started = *self.sleep_since.get_or_insert(now);
            self.sleep_step = ((now - started) / SLEEP_STEP_SECONDS) as usize % 4;
            self.pose = settings.size.pose([0, 1, 2, 1][self.sleep_step], true);
            self.state = "sleep";
            return &self.pose;
        }

        let was_sleeping = self.sleep_since.take().is_some();

        let previous_fps = self.adjust_speed(rate, elapsed, settings);
        if was_sleeping {
            self.frame = (self.frame + 1) % 5;
            self.phase = 0.0;
        } else {
            // Keep fractional progress between polls, and advance by elapsed
            // animation time if a slow redraw skipped one or more frames.
            let frames = self.phase + elapsed * (previous_fps + self.fps) * 0.5;
            self.frame = (self.frame + (frames % 5.0) as u32) % 5;
            self.phase = frames.fract();
        }
        self.pose = settings.size.pose(self.frame, false);
        self.state = "run";
        &self.pose
    }

    fn adjust_speed(&mut self, rate: f64, elapsed: f64, settings: Settings) -> f64 {
        let previous = self.fps;
        let target = settings.minimum + rate * 0.75;
        let smoothing = 1.0 - 0.15_f64.powf(elapsed / 0.3);
        self.fps = (self.fps + (target - self.fps) * smoothing).max(settings.minimum);
        if let Some(maximum) = settings.maximum {
            self.fps = self.fps.min(maximum);
        }
        previous
    }

    pub fn state(&self) -> &'static str {
        self.state
    }

    pub fn roam(&mut self, now: f64) -> (u16, bool, String) {
        let (position, left) = self.patrol.update(now, self.state, self.fps);
        let pose = if left && self.skin == Skin::PetCats {
            crate::pet::mirror(&self.pose)
        } else {
            self.pose.clone()
        };
        (position, left, pose)
    }

    pub fn next_update(&self) -> f64 {
        if self.skin == Skin::PetCats {
            self.pet.deadline
        } else if let Some(started) = self.sleep_since {
            let step = ((self.updated - started) / SLEEP_STEP_SECONDS).floor();
            started + (step + 1.0) * SLEEP_STEP_SECONDS
        } else {
            self.updated + (1.0 - self.phase) / self.fps
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleep_wake_and_two_second_timeout() {
        let mut cat = Animation::new(0, 0.0);
        let settings = Settings::default();
        assert_eq!(cat.update(0, 0.0, settings), SLEEP_POSE);
        assert_ne!(cat.update(1, 0.1, settings), SLEEP_POSE);
        assert_ne!(cat.update(1, 2.09, settings), SLEEP_POSE);
        assert_eq!(cat.update(1, 2.11, settings), SLEEP_POSE);
    }

    #[test]
    fn typing_accelerates_without_a_speed_limit() {
        let (mut slow, mut fast) = (Animation::new(0, 0.0), Animation::new(0, 0.0));
        let settings = Settings::default();
        for step in 1..=20 {
            slow.update(1 + step / 6, step as f64 * 0.05, settings);
            fast.update(step * 4, step as f64 * 0.05, settings);
        }
        assert!(fast.fps > slow.fps * 2.0);
        assert!(fast.fps > 50.0);
        assert!(fast.next_update() - 1.0 < 0.02);
    }

    #[test]
    fn fast_typing_produces_more_than_twenty_frames_per_second() {
        let mut cat = Animation::new(0, 0.0);
        let mut changes = 0;
        let mut previous = String::new();
        for step in 1..=3000 {
            let pose = cat.update(step / 10, step as f64 * 0.001, Settings::default());
            if step > 2000 && pose != previous {
                changes += 1;
            }
            previous = pose.to_owned();
        }
        assert!(
            changes > 60,
            "only {changes} frame changes during fast typing"
        );
    }

    #[test]
    fn explicit_limits_still_apply_and_can_be_removed_live() {
        let mut cat = Animation::new(0, 0.0);
        let capped = Settings::parse("2", "3", "12", "medium", "classic");
        for step in 1..=20 {
            cat.update(step * 4, step as f64 * 0.05, capped);
        }
        assert!(cat.fps <= 12.0);
        cat.update(84, 1.05, Settings::default());
        assert!(cat.fps > 12.0);
        assert_eq!(
            Settings::parse("2", "3", "120", "medium", "classic").maximum,
            Some(120.0)
        );
        for maximum in ["0", "unlimited", "NaN", "inf", "bad", "-1"] {
            assert_eq!(
                Settings::parse("2", "3", maximum, "medium", "classic").maximum,
                None
            );
        }
    }

    #[test]
    fn delayed_updates_preserve_frame_progress() {
        let mut cat = Animation::new(0, 0.0);
        let settings = Settings::parse("2", "10", "10", "medium", "classic");
        cat.update(1, 0.0, settings);
        assert_eq!(
            cat.update(1, 0.35, settings),
            CatSize::Medium.pose(3, false)
        );
        assert!((cat.next_update() - 0.4).abs() < 0.000_001);
    }

    #[test]
    fn slows_smoothly_before_sleeping() {
        let mut cat = Animation::new(0, 0.0);
        let settings = Settings::default();
        for step in 1..=20 {
            cat.update(step, step as f64 * 0.05, settings);
        }
        let peak = cat.fps;
        for step in 21..=50 {
            cat.update(20, step as f64 * 0.05, settings);
        }
        assert!(cat.fps < peak);
        assert_ne!(cat.pose, SLEEP_POSE);
    }

    #[test]
    fn counter_reset_does_not_create_activity() {
        let mut cat = Animation::new(100, 0.0);
        assert_eq!(cat.update(0, 0.1, Settings::default()), SLEEP_POSE);
        assert_ne!(cat.update(1, 0.2, Settings::default()), SLEEP_POSE);
    }

    #[test]
    fn sessions_have_independent_activity() {
        let (mut first, mut second) = (Animation::new(0, 0.0), Animation::new(0, 0.0));
        assert_ne!(first.update(1, 0.1, Settings::default()), SLEEP_POSE);
        assert_eq!(second.update(0, 0.1, Settings::default()), SLEEP_POSE);
    }

    #[test]
    fn settings_validate_and_apply_live() {
        let settings = Settings::parse("NaN", "bad", "-1", "invalid", "invalid");
        assert_eq!(
            (settings.idle, settings.minimum, settings.maximum),
            (2.0, 3.0, None)
        );
        let settings = Settings::parse("0.5", "8", "20", "medium", "classic");
        let mut cat = Animation::new(0, 0.0);
        cat.update(1, 0.1, settings);
        assert!(cat.fps >= 8.0);
        assert_eq!(cat.update(1, 0.61, settings), SLEEP_POSE);
    }

    #[test]
    fn sleeping_breathes_with_a_fixed_width_and_wakes_immediately() {
        let mut cat = Animation::new(0, 0.0);
        let settings = Settings::default();
        let expected = [0, 1, 2, 1, 0];
        for (step, frame) in expected.into_iter().enumerate() {
            let pose = cat.update(0, step as f64 * 0.75, settings);
            assert_eq!(pose, SLEEP_POSES[frame]);
            assert_eq!(pose.chars().count(), 2);
        }
        let pose = cat.update(1, 3.1, settings);
        assert_ne!(pose, SLEEP_POSE);
        assert_eq!(pose.chars().count(), 2);
    }

    #[test]
    fn all_sizes_keep_their_width_while_running_and_breathing() {
        for (size, width) in [
            (CatSize::Small, 1),
            (CatSize::Medium, 2),
            (CatSize::Large, 3),
            (CatSize::XLarge, 4),
        ] {
            let mut cat = Animation::new(0, 0.0);
            let settings = Settings {
                size,
                ..Settings::default()
            };
            for step in 0..4 {
                assert_eq!(
                    cat.update(0, step as f64 * 0.75, settings).chars().count(),
                    width
                );
            }
            assert_eq!(cat.update(1, 3.1, settings).chars().count(), width);
        }
    }

    #[test]
    fn size_updates_immediately_without_resetting_activity() {
        let mut cat = Animation::new(0, 0.0);
        cat.update(1, 0.1, Settings::default());
        let large = Settings {
            size: CatSize::Large,
            ..Settings::default()
        };
        assert_eq!(cat.update(1, 0.11, large).chars().count(), 3);
        let small = Settings {
            size: CatSize::Small,
            ..Settings::default()
        };
        assert_eq!(cat.update(1, 0.12, small).chars().count(), 1);
        assert_eq!(CatSize::parse(" LARGE "), CatSize::Large);
        assert_eq!(CatSize::parse("1"), CatSize::Small);
        assert_eq!(CatSize::parse("2"), CatSize::Medium);
        assert_eq!(CatSize::parse(" XLARGE "), CatSize::XLarge);
        assert_eq!(CatSize::parse("4"), CatSize::XLarge);
    }

    #[test]
    fn pet_cats_wake_walk_run_groom_lie_down_and_sleep() {
        let settings = Settings {
            skin: Skin::PetCats,
            size: CatSize::Large,
            ..Settings::default()
        };
        let mut cat = Animation::new(0, 0.0);
        cat.update(0, 0.0, settings);
        assert_eq!(cat.state(), "sleep");
        cat.update(1, 0.1, settings);
        assert_eq!(cat.state(), "stretch");
        cat.update(2, 0.4, settings);
        assert_eq!(
            cat.state(),
            "stretch",
            "typing restarted or interrupted waking"
        );
        cat.update(3, 0.76, settings);
        assert_eq!(cat.state(), "walk");
        cat.update(20, 0.8, settings);
        assert_eq!(cat.state(), "run");
        cat.update(20, 1.21, settings);
        assert_eq!(cat.state(), "lick-paw");
        cat.update(20, 2.01, settings);
        assert_eq!(cat.state(), "lie-down");
        let sleeping = cat.update(20, 2.81, settings).to_owned();
        assert_eq!(cat.state(), "sleep");
        assert_ne!(cat.update(20, 3.56, settings), sleeping);
        assert_eq!(cat.pose.chars().count(), 3);
        cat.update(20, 32.81, settings);
        assert_eq!(cat.state(), "deep-sleep");
        cat.update(21, 32.82, settings);
        assert_eq!(cat.state(), "stretch");
    }

    #[test]
    fn pet_cats_pause_gestures_are_varied_and_interruptible() {
        let settings = Settings {
            skin: Skin::PetCats,
            ..Settings::default()
        };
        let mut cat = Animation::new(0, 0.0);
        cat.update(1, 0.0, settings);
        let mut gestures = std::collections::HashSet::new();
        for step in 1..=6 {
            let now = step as f64 * 0.8;
            cat.update(step + 1, now, settings);
            assert_eq!(cat.state(), "walk");
            cat.update(step + 1, now + 0.4, settings);
            gestures.insert(cat.state());
        }
        assert_eq!(
            gestures,
            ["lick-paw", "idle", "itch", "lick-body", "meow", "sit"]
                .into_iter()
                .collect()
        );
        cat.update(7, 6.01, settings);
        assert_eq!(cat.state(), "lie-down");
        cat.update(8, 6.02, settings);
        assert_eq!(cat.state(), "walk", "lying down delayed a new keypress");
    }

    #[test]
    fn pet_cats_preserve_uncapped_speed_and_live_skin_changes() {
        let mut cat = Animation::new(0, 0.0);
        let settings = Settings {
            skin: Skin::PetCats,
            ..Settings::default()
        };
        let mut changes = 0;
        let mut previous = String::new();
        for step in 1..=3000 {
            let pose = cat.update(step / 10, step as f64 * 0.001, settings);
            if step > 2000 && pose != previous {
                changes += 1;
            }
            previous = pose.to_owned();
        }
        assert!(changes > 60);
        assert_eq!(cat.state(), "run");
        assert!(cat.fps > 60.0);
        let classic = cat.update(300, 3.01, Settings::default());
        assert!(
            classic
                .chars()
                .all(|value| (0xeff0..=0xeff9).contains(&(value as u32)))
        );
        let pet = cat.update(301, 3.02, settings);
        assert!(
            pet.chars()
                .all(|value| (0xf700..0xf8a4).contains(&(value as u32)))
        );
        assert_eq!(Skin::parse(" PET-CATS "), Skin::PetCats);
        assert_eq!(Skin::parse("invalid"), Skin::Classic);
    }

    #[test]
    fn spinning_continues_while_typing_then_stops_on_pause() {
        let settings = Settings {
            skin: Skin::PetCats,
            spin_chance: 100.0,
            ..Settings::default()
        };
        let mut cat = Animation::new(0, 0.0);
        cat.update(1, 0.1, settings);
        cat.update(20, 0.8, settings);
        assert_eq!(cat.state(), "run");
        let first = cat.update(40, 1.1, settings).to_owned();
        assert_eq!(cat.state(), "spin");
        assert!(
            first
                .chars()
                .all(|c| (0x100000..0x1000a0).contains(&(c as u32)))
        );
        assert_ne!(cat.update(50, 1.2, settings), first);
        assert_eq!(cat.state(), "spin");
        for step in 0..30 {
            cat.update(60 + step * 10, 1.3 + step as f64 * 0.1, settings);
            assert_eq!(cat.state(), "spin", "spin stopped while keys kept arriving");
        }
        cat.update(350, 4.57, settings);
        assert_ne!(cat.state(), "spin", "spin continued after typing paused");
        cat.update(370, 4.6, settings);
        assert_eq!(cat.state(), "run", "spin cooldown was ignored");
        cat.update(370, 6.61, settings);
        assert_eq!(cat.state(), "sleep");
        assert_eq!(Settings::default().spin_chance, 0.0);
        assert_eq!(Settings::default().with_spin_chance("NaN").spin_chance, 0.0);
        assert_eq!(
            Settings::default().with_spin_chance("200").spin_chance,
            100.0
        );
    }

    #[test]
    fn spin_accelerates_with_typing_and_keeps_fractional_progress() {
        let settings = Settings {
            skin: Skin::PetCats,
            spin_chance: 100.0,
            ..Settings::default()
        };
        let mut slow = Animation::new(0, 0.0);
        let mut fast = Animation::new(0, 0.0);
        for cat in [&mut slow, &mut fast] {
            cat.update(1, 0.1, settings);
        }
        slow.update(20, 0.8, settings);
        fast.update(100, 0.8, settings);
        slow.update(40, 1.1, settings);
        fast.update(200, 1.1, settings);
        assert_eq!(slow.state(), "spin");
        assert_eq!(fast.state(), "spin");
        assert!(fast.next_update() - 1.1 < (slow.next_update() - 1.1) * 0.5);

        let first = fast.update(220, 1.105, settings).to_owned();
        let second = fast.update(240, 1.11, settings).to_owned();
        assert_ne!(
            first, second,
            "spin frames did not advance at high typing speed"
        );
    }

    #[test]
    fn roaming_turns_the_pet_to_face_its_travel_direction() {
        let settings = Settings {
            skin: Skin::PetCats,
            ..Settings::default()
        };
        let mut cat = Animation::new(0, 0.0);
        cat.update(1, 0.0, settings);
        cat.roam(0.0);
        cat.update(100, 4.0, settings);
        let (_, left, pose) = cat.roam(4.0);
        assert!(left);
        assert!(
            pose.chars()
                .all(|c| (0x101000..0x10135c).contains(&(c as u32)))
        );
    }
}
