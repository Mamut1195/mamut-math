//! A scripted-uniform `RngPort` double for component conformance tests.

use mamut_math_core::{RngMetadata, RngPort, RNG_ALGORITHM_ID, RNG_ALGORITHM_VERSION};

/// An `RngPort` whose `next_unit_f64` returns a fixed `units` queue in order.
///
/// Used to feed the SAME uniform stream to the crate's genetic operators and to
/// the DEAP oracle (driven from the committed fixture). `next_u64`/`next_u32`/
/// `fill_bytes` are backed by a separate word queue; the component fixtures only
/// exercise `next_unit_f64`.
pub struct ScriptedUniform {
    units: Vec<f64>,
    words: Vec<u64>,
    unit_pos: usize,
    word_pos: usize,
}

impl ScriptedUniform {
    pub fn new(units: Vec<f64>) -> Self {
        // One word per unit is plenty for the operators under test.
        let words = (0..units.len() as u64 + 8)
            .map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15))
            .collect();
        Self {
            units,
            words,
            unit_pos: 0,
            word_pos: 0,
        }
    }
}

impl RngPort for ScriptedUniform {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        let value = self.words[self.word_pos];
        self.word_pos += 1;
        value
    }

    fn next_unit_f64(&mut self) -> f64 {
        let value = self.units[self.unit_pos];
        self.unit_pos += 1;
        value
    }

    fn fill_bytes(&mut self, dst: &mut [u8]) {
        for chunk in dst.chunks_mut(8) {
            let bytes = self.next_u64().to_le_bytes();
            for (slot, byte) in chunk.iter_mut().zip(bytes) {
                *slot = byte;
            }
        }
    }

    fn metadata(&self) -> RngMetadata {
        RngMetadata::new(
            RNG_ALGORITHM_ID,
            RNG_ALGORITHM_VERSION,
            [0u8; 8],
            Vec::new(),
            0,
        )
        .expect("frozen algorithm id is valid")
    }

    fn derive_child_port(&self, _stream_index: u64) -> Box<dyn RngPort> {
        Box::new(ScriptedUniform::new(self.units.clone()))
    }
}
