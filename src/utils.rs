use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::filter::threshold::ThresholdFilter;
use std::f32::consts::PI;

pub fn init_log4rs() {
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build("log/output.log")
        .expect("Failed to create log file appender");

    let console = ConsoleAppender::builder().build();

    let config = Config::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(LevelFilter::Info)))
                .build("logfile", Box::new(logfile)),
        )
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(LevelFilter::Debug)))
                .build("stdout", Box::new(console)),
        )
        .build(
            Root::builder()
                .appender("logfile")
                .appender("stdout")
                .build(LevelFilter::Debug),
        )
        .expect("Failed to create log config");

    log4rs::init_config(config).expect("Failed to initialize log4rs");
}

// AngleIter from https://vcs.cozydsp.space/cozy-dsp/cozy-ui/src/commit/d4706ec9f4592137307ce8acafb56b881ea54e35/src/util.rs#L49

const PI_OVER_2: f32 = PI / 2.0;

/// An iterator that yields pairs of angles to build a full circle.
pub struct AngleIter {
    start: Option<f32>,
    end: f32,
}

impl AngleIter {
    pub const fn new(start_angle: f32, end_angle: f32) -> Self {
        Self {
            start: Some(start_angle),
            end: end_angle,
        }
    }
}

impl Iterator for AngleIter {
    type Item = (f32, f32);

    fn next(&mut self) -> Option<Self::Item> {
        self.start.map(|start| {
            let diff = self.end - start;
            if diff.abs() < PI_OVER_2 {
                self.start = None;
                (start, self.end)
            } else {
                let new_start = start + (PI_OVER_2 * diff.signum());
                self.start = Some(new_start);
                (start, new_start)
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            0,
            self.start
                .map(|start| ((self.end - start).abs() / PI_OVER_2).ceil() as usize),
        )
    }
}

pub fn capitalize_first_letter(s: &str) -> String {
    let mut character_iter = s.chars();
    match character_iter.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + character_iter.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitalize_first_letter_empty() {
        assert_eq!(capitalize_first_letter(""), "");
    }

    #[test]
    fn capitalize_first_letter_single_char() {
        assert_eq!(capitalize_first_letter("a"), "A");
    }

    #[test]
    fn capitalize_first_letter_already_upper() {
        assert_eq!(capitalize_first_letter("Hello"), "Hello");
    }

    #[test]
    fn capitalize_first_letter_lowercase() {
        assert_eq!(capitalize_first_letter("hello world"), "Hello world");
    }

    #[test]
    fn capitalize_first_letter_unicode() {
        assert_eq!(capitalize_first_letter("über"), "Über");
    }

    #[test]
    fn angle_iter_zero_length_arc() {
        let iter = AngleIter::new(1.0, 1.0);
        let pairs: Vec<_> = iter.collect();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], (1.0, 1.0));
    }

    #[test]
    fn angle_iter_small_arc() {
        let iter = AngleIter::new(0.0, 1.0);
        let pairs: Vec<_> = iter.collect();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], (0.0, 1.0));
    }

    #[test]
    fn angle_iter_full_circle() {
        let iter = AngleIter::new(0.0, 2.0 * PI);
        let pairs: Vec<_> = iter.collect();
        assert!(
            pairs.len() >= 4,
            "full circle should produce at least 4 segments"
        );
        assert_eq!(pairs.first().unwrap().0, 0.0);
        assert!((pairs.last().unwrap().1 - 2.0 * PI).abs() < 1e-5);
    }

    #[test]
    fn angle_iter_negative_direction() {
        let iter = AngleIter::new(0.0, -PI);
        let pairs: Vec<_> = iter.collect();
        assert!(pairs.len() >= 2);
        assert_eq!(pairs.first().unwrap().0, 0.0);
        assert!((pairs.last().unwrap().1 - (-PI)).abs() < 1e-5);
    }

    #[test]
    fn angle_iter_size_hint_upper_bound() {
        let iter = AngleIter::new(0.0, 2.0 * PI);
        let (lower, upper) = iter.size_hint();
        assert_eq!(lower, 0);
        let upper = upper.unwrap();
        assert!(upper >= 4);
    }

    #[test]
    fn angle_iter_segments_are_contiguous() {
        let iter = AngleIter::new(0.0, PI);
        let pairs: Vec<_> = iter.collect();
        for window in pairs.windows(2) {
            assert!(
                (window[0].1 - window[1].0).abs() < 1e-5,
                "segment end should equal next segment start"
            );
        }
    }
}
