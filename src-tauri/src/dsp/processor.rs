pub trait AudioProcessor: Send {
    fn prepare(&mut self, sample_rate: u32, channels: usize, block_size: usize);

    fn process(&mut self, samples: &mut [f32]);

    fn reset(&mut self);
}
