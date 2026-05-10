//! Sample-accurate parameter ramps.

use std::collections::HashMap;

/// Key-value map of parameter names to current float values.
pub type ParamMap = HashMap<String, f32>;

/// A smoothed parameter ramp (linear interpolation per sample).
#[derive(Debug, Clone)]
pub struct ParamRamp {
    current: f32,
    target: f32,
    step: f32,
    samples_remaining: usize,
}

impl ParamRamp {
    pub fn new(initial: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            step: 0.0,
            samples_remaining: 0,
        }
    }

    /// Set a new target value to be reached over `samples` samples.
    pub fn ramp_to(&mut self, target: f32, samples: usize) {
        self.target = target;
        if samples == 0 {
            self.current = target;
            self.step = 0.0;
            self.samples_remaining = 0;
        } else {
            self.step = (target - self.current) / samples as f32;
            self.samples_remaining = samples;
        }
    }

    /// Instant jump.
    pub fn set(&mut self, value: f32) {
        self.current = value;
        self.target = value;
        self.step = 0.0;
        self.samples_remaining = 0;
    }

    /// Advance one sample and return the current value.
    pub fn next(&mut self) -> f32 {
        if self.samples_remaining > 0 {
            self.current += self.step;
            self.samples_remaining -= 1;
            if self.samples_remaining == 0 {
                self.current = self.target;
            }
        }
        self.current
    }

    /// Fill a buffer with ramped values.
    pub fn fill_buffer(&mut self, buf: &mut [f32]) {
        for v in buf.iter_mut() {
            *v = self.next();
        }
    }
}

/// Collection of ramps keyed by parameter name.
#[derive(Debug, Default)]
pub struct ParamRamps {
    ramps: HashMap<String, ParamRamp>,
}

impl ParamRamps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, key: &str, initial: f32) {
        self.ramps
            .insert(key.to_string(), ParamRamp::new(initial));
    }

    pub fn ramp(&mut self, key: &str, target: f32, samples: usize) {
        if let Some(r) = self.ramps.get_mut(key) {
            r.ramp_to(target, samples);
        }
    }

    pub fn set(&mut self, key: &str, value: f32) {
        if let Some(r) = self.ramps.get_mut(key) {
            r.set(value);
        }
    }

    pub fn next(&mut self, key: &str) -> f32 {
        self.ramps.get_mut(key).map(|r| r.next()).unwrap_or(0.0)
    }

    pub fn snapshot(&self, key: &str) -> f32 {
        self.ramps.get(key).map(|r| r.current).unwrap_or(0.0)
    }
}
