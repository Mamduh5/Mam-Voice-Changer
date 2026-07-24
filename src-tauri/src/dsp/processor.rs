pub trait AudioProcessor: Send {
    fn prepare(
        &mut self,
        sample_rate: u32,
        channels: usize,
        maximum_block_size: usize,
    ) -> Result<(), String>;

    fn process(&mut self, samples: &mut [f32]);

    fn reset(&mut self);
}
