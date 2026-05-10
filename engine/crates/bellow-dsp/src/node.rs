//! Core `Node` trait implemented by every DSP processor.

use crate::param::ParamMap;

/// A real-time DSP node.
///
/// `input` and `output` are interleaved f32 buffers of `channels` × `frames`.
/// `params` holds sample-accurate parameter values for this block.
pub trait Node: Send {
    fn process(&mut self, input: &[f32], output: &mut [f32], params: &ParamMap);

    fn reset(&mut self) {}

    fn channels(&self) -> u16 {
        2
    }

    fn name(&self) -> &str;
}

/// A pass-through node (useful for testing / bypass).
pub struct BypassNode;

impl Node for BypassNode {
    fn process(&mut self, input: &[f32], output: &mut [f32], _params: &ParamMap) {
        output.copy_from_slice(input);
    }

    fn name(&self) -> &str {
        "bypass"
    }
}
