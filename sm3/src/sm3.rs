use block_buffer::byteorder::{ByteOrder, BE};
use block_buffer::BlockBuffer;
use digest::generic_array::typenum::{U32, U64};
use digest::generic_array::GenericArray;
use digest::{BlockInput, FixedOutput, Input, Reset};

use crate::consts::{IV, STATE_LEN};
use crate::utils::compress256;

type BlockSize = U64;
type Block = GenericArray<u8, BlockSize>;

#[derive(Clone)]
struct EngineState {
    h: [u32; 8],
}

impl EngineState {
    fn new(h: &[u32; STATE_LEN]) -> EngineState {
        EngineState { h: *h }
    }

    pub fn process_block(&mut self, block: &Block) {
        let block = unsafe { &*(block.as_ptr() as *const [u8; 64]) };
        compress256(&mut self.h, block);
    }
}

#[derive(Clone)]
struct Engine {
    len: u64,
    buffer: BlockBuffer<BlockSize>,
    state: EngineState,
}

impl Engine {
    fn new(h: &[u32; STATE_LEN]) -> Engine {
        Engine {
            len: 0,
            buffer: Default::default(),
            state: EngineState::new(h),
        }
    }

    fn input(&mut self, input: &[u8]) {
        // Assumes that input.len() can be converted to u64 without overflow
        self.len += (input.len() as u64) << 3;
        let self_state = &mut self.state;
        self.buffer
            .input(input, |input| self_state.process_block(input));
    }

    fn finish(&mut self) {
        let self_state = &mut self.state;
        let l = self.len;
        self.buffer
            .len64_padding::<BE, _>(l, |b| self_state.process_block(b));
    }

    fn reset(&mut self, h: &[u32; STATE_LEN]) {
        self.len = 0;
        self.buffer.reset();
        self.state = EngineState::new(h);
    }
}

#[derive(Clone)]
pub struct Sm3 {
    engine: Engine,
}

impl Default for Sm3 {
    fn default() -> Self {
        Sm3 {
            engine: Engine::new(&IV),
        }
    }
}

impl BlockInput for Sm3 {
    type BlockSize = BlockSize;
}

impl Input for Sm3 {
    fn input<B: AsRef<[u8]>>(&mut self, input: B) {
        self.engine.input(input.as_ref());
    }
}

impl FixedOutput for Sm3 {
    type OutputSize = U32;

    fn fixed_result(mut self) -> GenericArray<u8, Self::OutputSize> {
        self.engine.finish();
        let mut out = GenericArray::default();
        BE::write_u32_into(&self.engine.state.h, out.as_mut_slice());
        out
    }
}

impl Reset for Sm3 {
    fn reset(&mut self) {
        self.engine.reset(&IV);
    }
}

#[cfg(test)]
mod tests {
    use super::Sm3;
    use digest::Digest;

    #[test]
    fn example_1() {
        let string = "abc".to_owned();
        let s = string.as_bytes();

        let mut hasher = Sm3::new();
        hasher.input(s);
        assert_eq!(
            format!("{:x}", hasher.result()),
            "66c7f0f462eeedd9d1f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba8e0"
        );
    }

    #[test]
    fn example_2() {
        let string = "abcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcd".to_owned();
        let s = string.as_bytes();

        let mut hasher = Sm3::new();
        hasher.input(s);
        assert_eq!(
            format!("{:x}", hasher.result()),
            "debe9ff92275b8a138604889c18e5a4d6fdb70e5387e5765293dcba39c0c5732"
        );
    }
}
