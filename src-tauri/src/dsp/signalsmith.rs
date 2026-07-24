use std::{ffi::c_int, ptr::NonNull};

enum MamSignalsmithStretch {}

extern "C" {
    fn mam_signalsmith_create(
        channels: c_int,
        block_frames: usize,
        interval_frames: usize,
    ) -> *mut MamSignalsmithStretch;
    fn mam_signalsmith_destroy(handle: *mut MamSignalsmithStretch);
    fn mam_signalsmith_reset(handle: *mut MamSignalsmithStretch);
    fn mam_signalsmith_input_latency(handle: *mut MamSignalsmithStretch) -> usize;
    fn mam_signalsmith_output_latency(handle: *mut MamSignalsmithStretch) -> usize;
    fn mam_signalsmith_set_pitch_semitones(handle: *mut MamSignalsmithStretch, semitones: f32);
    fn mam_signalsmith_set_formant_semitones(
        handle: *mut MamSignalsmithStretch,
        semitones: f32,
        compensate_pitch: bool,
    );
    fn mam_signalsmith_process(
        handle: *mut MamSignalsmithStretch,
        input: *mut f32,
        input_frames: usize,
        output: *mut f32,
        output_frames: usize,
    ) -> bool;
}

pub struct SignalsmithStretch {
    handle: NonNull<MamSignalsmithStretch>,
    channels: usize,
}

unsafe impl Send for SignalsmithStretch {}

impl SignalsmithStretch {
    pub fn new(
        channels: usize,
        block_frames: usize,
        interval_frames: usize,
    ) -> Result<Self, String> {
        if channels == 0 {
            return Err("Signalsmith requires at least one audio channel.".to_owned());
        }
        if block_frames == 0 || interval_frames == 0 || interval_frames > block_frames {
            return Err(
                "Signalsmith requires a nonzero interval no larger than its analysis block."
                    .to_owned(),
            );
        }
        let channels = c_int::try_from(channels)
            .map_err(|_| "Signalsmith channel count exceeds the native ABI limit.".to_owned())?;
        let handle = unsafe { mam_signalsmith_create(channels, block_frames, interval_frames) };
        NonNull::new(handle)
            .map(|handle| Self {
                handle,
                channels: channels as usize,
            })
            .ok_or_else(|| "Signalsmith could not allocate or configure its backend.".to_owned())
    }

    pub fn reset(&mut self) {
        unsafe { mam_signalsmith_reset(self.handle.as_ptr()) }
    }

    pub fn input_latency(&self) -> usize {
        unsafe { mam_signalsmith_input_latency(self.handle.as_ptr()) }
    }

    pub fn output_latency(&self) -> usize {
        unsafe { mam_signalsmith_output_latency(self.handle.as_ptr()) }
    }

    pub fn set_pitch_semitones(&mut self, semitones: f32) {
        unsafe { mam_signalsmith_set_pitch_semitones(self.handle.as_ptr(), semitones) }
    }

    pub fn set_formant_semitones(&mut self, semitones: f32, compensate_pitch: bool) {
        unsafe {
            mam_signalsmith_set_formant_semitones(self.handle.as_ptr(), semitones, compensate_pitch)
        }
    }

    pub fn process(&mut self, input: &mut [f32], output: &mut [f32]) -> Result<(), &'static str> {
        if input.len() != output.len()
            || !input.len().is_multiple_of(self.channels)
            || !output.len().is_multiple_of(self.channels)
        {
            output.fill(0.0);
            return Err("Signalsmith input and output must contain equal complete frames.");
        }
        let processed = unsafe {
            mam_signalsmith_process(
                self.handle.as_ptr(),
                input.as_mut_ptr(),
                input.len() / self.channels,
                output.as_mut_ptr(),
                output.len() / self.channels,
            )
        };
        if processed {
            Ok(())
        } else {
            output.fill(0.0);
            Err("Signalsmith rejected the processing block.")
        }
    }
}

impl Drop for SignalsmithStretch {
    fn drop(&mut self) {
        unsafe { mam_signalsmith_destroy(self.handle.as_ptr()) }
    }
}
