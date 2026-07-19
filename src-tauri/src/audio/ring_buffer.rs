use ringbuf::{
    traits::{Observer, Producer, Split},
    HeapCons, HeapProd, HeapRb,
};

#[cfg(test)]
use ringbuf::traits::Consumer;

pub struct AudioRingBuffer {
    producer: HeapProd<f32>,
    consumer: HeapCons<f32>,
}

impl AudioRingBuffer {
    pub fn new(capacity_samples: usize, prefill_samples: usize) -> Self {
        let buffer = HeapRb::<f32>::new(capacity_samples.max(1));
        let (mut producer, consumer) = buffer.split();
        for _ in 0..prefill_samples.min(capacity_samples) {
            let _ = producer.try_push(0.0);
        }
        Self { producer, consumer }
    }

    pub fn split(self) -> (HeapProd<f32>, HeapCons<f32>) {
        (self.producer, self.consumer)
    }
}

pub fn push_or_drop_newest(producer: &mut HeapProd<f32>, sample: f32) -> bool {
    producer.try_push(sample).is_ok()
}

pub fn push_frame_or_drop(producer: &mut HeapProd<f32>, frame: &[f32]) -> bool {
    if producer.vacant_len() < frame.len() {
        return false;
    }
    for sample in frame {
        if producer.try_push(*sample).is_err() {
            return false;
        }
    }
    true
}

#[cfg(test)]
fn pop_or_silence(consumer: &mut HeapCons<f32>) -> (f32, bool) {
    match consumer.try_pop() {
        Some(sample) => (sample, false),
        None => (0.0, true),
    }
}

#[cfg(test)]
mod tests {
    use ringbuf::traits::Consumer;

    use super::{pop_or_silence, push_frame_or_drop, push_or_drop_newest, AudioRingBuffer};

    #[test]
    fn underflow_returns_silence() {
        let (_, mut consumer) = AudioRingBuffer::new(2, 0).split();
        assert_eq!(pop_or_silence(&mut consumer), (0.0, true));
    }

    #[test]
    fn overflow_drops_the_newest_sample() {
        let (mut producer, mut consumer) = AudioRingBuffer::new(1, 0).split();
        assert!(push_or_drop_newest(&mut producer, 0.25));
        assert!(!push_or_drop_newest(&mut producer, 0.75));
        assert_eq!(pop_or_silence(&mut consumer), (0.25, false));
    }

    #[test]
    fn complete_frames_are_dropped_atomically() {
        let (mut producer, mut consumer) = AudioRingBuffer::new(3, 0).split();
        assert!(push_frame_or_drop(&mut producer, &[0.1, 0.2]));
        assert!(!push_frame_or_drop(&mut producer, &[0.3, 0.4]));
        assert_eq!(consumer.try_pop(), Some(0.1));
        assert_eq!(consumer.try_pop(), Some(0.2));
        assert_eq!(consumer.try_pop(), None);
    }
}
