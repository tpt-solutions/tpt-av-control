//! Parameter mapping primitives: identifiers, values, transfer curves, and
//! automation.

use crate::time::Timestamp;

/// A dotted parameter path, e.g. `track.1.volume`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ParameterId(String);

impl ParameterId {
    /// Creates a parameter id from a dotted path.
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    /// The path as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The path segments split on `.`.
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('.')
    }
}

impl std::fmt::Display for ParameterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A parameter value.
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterValue {
    /// Floating-point value (most control parameters).
    Float(f32),
    /// Integer value (e.g. preset numbers).
    Int(i64),
    /// Boolean value (mutes, play states).
    Bool(bool),
    /// Text value (labels, scene names).
    String(String),
}

impl ParameterValue {
    /// Numeric view of the value for mapping math. `Bool` maps to 0/1;
    /// `String` has no numeric view and yields `None`.
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            ParameterValue::Float(v) => Some(*v),
            ParameterValue::Int(v) => Some(*v as f32),
            ParameterValue::Bool(v) => Some(if *v { 1.0 } else { 0.0 }),
            ParameterValue::String(_) => None,
        }
    }
}

/// A normalized transfer curve applied to the 0..1 mapping interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Curve {
    /// Identity.
    Linear,
    /// Quadratic ease-in: t².
    EaseIn,
    /// Quadratic ease-out: 1-(1-t)².
    EaseOut,
    /// Cubic ease-in-out (smoothstep): 3t²-2t³.
    EaseInOut,
    /// Exponential response: (e^(r·t)-1)/(e^r-1). `rate` > 0; higher
    /// rates give finer resolution near the bottom of the range.
    Exponential {
        /// Steepness of the exponential; must be > 0.
        rate: f32,
    },
    /// Logarithmic response: ln(r·t+1)/ln(r+1). `rate` > 0; higher
    /// rates give finer resolution near the top of the range.
    Logarithmic {
        /// Steepness of the logarithm; must be > 0.
        rate: f32,
    },
}

impl Curve {
    /// Applies the curve to `t`, clamped to 0..1.
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match *self {
            Curve::Linear => t,
            Curve::EaseIn => t * t,
            Curve::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Curve::EaseInOut => t * t * (3.0 - 2.0 * t),
            Curve::Exponential { rate } => {
                let r = rate.max(f32::EPSILON);
                ((r * t).exp() - 1.0) / (r.exp() - 1.0)
            }
            Curve::Logarithmic { rate } => {
                let r = rate.max(f32::EPSILON);
                (r.mul_add(t, 1.0)).ln() / (r + 1.0).ln()
            }
        }
    }
}

/// Maps one numeric range onto another through a transfer curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mapping {
    /// Input range (min, max), e.g. a MIDI CC range of (0.0, 127.0).
    pub input_range: (f32, f32),
    /// Output range (min, max), e.g. a parameter range of (-60.0, 6.0) dB.
    pub output_range: (f32, f32),
    /// Transfer curve applied to the normalized input.
    pub curve: Curve,
}

impl Default for Mapping {
    fn default() -> Self {
        Self {
            input_range: (0.0, 1.0),
            output_range: (0.0, 1.0),
            curve: Curve::Linear,
        }
    }
}

impl Mapping {
    /// Creates a linear mapping between the two ranges.
    pub fn linear(input_range: (f32, f32), output_range: (f32, f32)) -> Self {
        Self {
            input_range,
            output_range,
            curve: Curve::Linear,
        }
    }

    /// Maps `input` through the curve onto the output range, clamped.
    pub fn map(&self, input: f32) -> f32 {
        let (in_min, in_max) = self.input_range;
        let (out_min, out_max) = self.output_range;
        let span = in_max - in_min;
        let t = if span.abs() < f32::EPSILON {
            0.0
        } else {
            (input - in_min) / span
        };
        let shaped = self.curve.apply(t);
        out_min + shaped * (out_max - out_min)
    }
}

/// A keyframe on an [`Automation`] timeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutomationPoint {
    /// Time of the keyframe in seconds along the timeline.
    pub time: f64,
    /// Value at the keyframe (in output units).
    pub value: f32,
    /// Curve shape used to travel *from* this point to the next.
    pub curve: Curve,
}
/// A keyframe automation timeline for one parameter.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Automation {
    points: Vec<AutomationPoint>,
}

impl Automation {
    /// An empty timeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a keyframe. Points need not be inserted in time order.
    pub fn add_point(&mut self, point: AutomationPoint) {
        let pos = self
            .points
            .partition_point(|existing| existing.time <= point.time);
        self.points.insert(pos, point);
    }

    /// All keyframes in time order.
    pub fn points(&self) -> &[AutomationPoint] {
        &self.points
    }

    /// Evaluates the automation at `time`. Before the first keyframe the
    /// first value holds; after the last, the last value holds; between
    /// keyframes the segment's curve interpolates.
    pub fn value_at(&self, time: f64) -> Option<f32> {
        match self.points.len() {
            0 => None,
            1 => Some(self.points[0].value),
            _ => {
                let first = &self.points[0];
                if time <= first.time {
                    return Some(first.value);
                }
                let last = &self.points[self.points.len() - 1];
                if time >= last.time {
                    return Some(last.value);
                }
                let idx = self.points.partition_point(|p| p.time <= time);
                let a = &self.points[idx - 1];
                let b = &self.points[idx];
                let span = b.time - a.time;
                let t = if span <= f64::EPSILON {
                    1.0
                } else {
                    ((time - a.time) / span) as f32
                };
                let shaped = a.curve.apply(t);
                Some(a.value + shaped * (b.value - a.value))
            }
        }
    }

    /// Evaluates using wall-clock time relative to `start`.
    pub fn value_at_timestamp(&self, start: Timestamp, now: Timestamp) -> Option<f32> {
        let seconds = now.as_nanos().saturating_sub(start.as_nanos()) as f64 / 1e9;
        self.value_at(seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_endpoints_hold() {
        for curve in [
            Curve::Linear,
            Curve::EaseIn,
            Curve::EaseOut,
            Curve::EaseInOut,
            Curve::Exponential { rate: 4.0 },
            Curve::Logarithmic { rate: 4.0 },
        ] {
            assert!((curve.apply(0.0) - 0.0).abs() < 1e-6, "{curve:?}");
            assert!((curve.apply(1.0) - 1.0).abs() < 1e-6, "{curve:?}");
            let mid = curve.apply(0.5);
            assert!((0.0..=1.0).contains(&mid), "{curve:?}");
        }
    }

    #[test]
    fn curve_monotonic() {
        let curve = Curve::Exponential { rate: 3.0 };
        let mut prev = 0.0;
        for step in 1..=100 {
            let v = curve.apply(step as f32 / 100.0);
            assert!(v >= prev, "non-monotonic at {step}");
            prev = v;
        }
    }

    #[test]
    fn linear_mapping_scales() {
        let m = Mapping::linear((0.0, 127.0), (-60.0, 0.0));
        assert!((m.map(0.0) - -60.0).abs() < 1e-5);
        assert!((m.map(127.0) - 0.0).abs() < 1e-5);
        assert!((m.map(63.5) - -30.0).abs() < 0.01);
        assert!((m.map(-10.0) - -60.0).abs() < 1e-5, "clamps low");
        assert!((m.map(200.0) - 0.0).abs() < 1e-5, "clamps high");
    }

    #[test]
    fn curved_mapping_bends() {
        let m = Mapping {
            curve: Curve::Exponential { rate: 2.0 },
            ..Mapping::default()
        };
        let mid = m.map(0.5);
        assert!(mid < 0.5, "exponential keeps resolution low: {mid}");
        let m = Mapping {
            curve: Curve::Logarithmic { rate: 2.0 },
            ..Mapping::default()
        };
        let mid = m.map(0.5);
        assert!(mid > 0.5, "logarithmic keeps resolution high: {mid}");
    }

    #[test]
    fn automation_interpolates() {
        let mut auto = Automation::new();
        auto.add_point(AutomationPoint {
            time: 2.0,
            value: 1.0,
            curve: Curve::Linear,
        });
        auto.add_point(AutomationPoint {
            time: 0.0,
            value: 0.0,
            curve: Curve::Linear,
        });
        assert_eq!(auto.value_at(-1.0), Some(0.0), "holds before start");
        assert_eq!(auto.value_at(5.0), Some(1.0), "holds after end");
        assert!((auto.value_at(1.0).unwrap() - 0.5).abs() < 1e-6);
        // Points() returns time-sorted.
        assert_eq!(auto.points()[0].time, 0.0);
    }

    #[test]
    fn automation_segment_curves_apply() {
        let mut auto = Automation::new();
        auto.add_point(AutomationPoint {
            time: 1.0,
            value: 1.0,
            curve: Curve::EaseIn,
        });
        auto.add_point(AutomationPoint {
            time: 0.0,
            value: 0.0,
            curve: Curve::EaseIn,
        });
        // t=0.5 through EaseIn: value = 0.25.
        assert!((auto.value_at(0.5).unwrap() - 0.25).abs() < 1e-6);
    }

    #[test]
    fn parameter_ids_segment() {
        let id = ParameterId::new("track.1.volume");
        assert_eq!(id.to_string(), "track.1.volume");
        assert_eq!(id.segments().count(), 3);
    }
}
