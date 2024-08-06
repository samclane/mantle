// AngleIter from https://vcs.cozydsp.space/cozy-dsp/cozy-ui/src/commit/d4706ec9f4592137307ce8acafb56b881ea54e35/src/util.rs#L49
use std::f32::consts::PI;

const PI_OVER_2: f32 = PI / 2.0;

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
