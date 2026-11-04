use std::sync::OnceLock;

struct Frame {
    phase: &'static str,
    end: f64,
}

fn frames() -> &'static [Frame] {
    static FRAMES: OnceLock<Vec<Frame>> = OnceLock::new();
    FRAMES.get_or_init(|| {
        let mut milliseconds = 0_u32;
        include_str!("../fonts/pet-cats/parkour.tsv")
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let (phase, duration) = line.split_once('\t').expect("parkour phase and duration");
                milliseconds += duration
                    .parse::<u32>()
                    .expect("parkour duration in milliseconds");
                Frame {
                    phase,
                    end: f64::from(milliseconds) / 1000.0,
                }
            })
            .collect()
    })
}

pub fn duration() -> f64 {
    frames().last().expect("parkour frames").end
}

pub fn phase_start(phase: &str) -> f64 {
    let index = frames()
        .iter()
        .position(|frame| frame.phase == phase)
        .expect("known parkour phase");
    if index == 0 {
        0.0
    } else {
        frames()[index - 1].end
    }
}

pub fn sample(age: f64) -> (u32, f64) {
    let index = frames()
        .partition_point(|frame| age >= frame.end)
        .min(frames().len() - 1);
    (index as u32, frames()[index].end)
}
