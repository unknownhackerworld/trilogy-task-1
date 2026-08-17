use anyhow::Result;
use rubato::{FftFixedIn, Resampler as RubatoResampler};

/// Resamples audio from native device rate to 16kHz mono PCM (required by ASR engines).
pub struct Resampler {
    resampler: FftFixedIn<f32>,
    input_channels: usize,
    chunk_size: usize,
}

impl Resampler {
    /// Create a resampler from source_rate/channels to 16kHz mono.
    pub fn new(source_rate: u32, source_channels: u16) -> Result<Self> {
        let target_rate = 16000;
        let chunk_size = (source_rate as usize) / 10; // 100ms chunks

        let resampler = FftFixedIn::<f32>::new(
            source_rate as usize,
            target_rate,
            chunk_size,
            2,
            source_channels as usize,
        )?;

        Ok(Self {
            resampler,
            input_channels: source_channels as usize,
            chunk_size,
        })
    }

    /// Resample interleaved i16 PCM to 16kHz mono i16 PCM.
    pub fn process(&mut self, input: &[i16]) -> Result<Vec<i16>> {
        let num_frames = input.len() / self.input_channels;

        // Deinterleave and convert to f32
        let mut channels: Vec<Vec<f32>> = vec![Vec::with_capacity(num_frames); self.input_channels];
        for (i, sample) in input.iter().enumerate() {
            let ch = i % self.input_channels;
            channels[ch].push(*sample as f32 / 32768.0);
        }

        // Pad to chunk_size if needed
        for ch in channels.iter_mut() {
            while ch.len() < self.chunk_size {
                ch.push(0.0);
            }
        }

        // Resample
        let output = self.resampler.process(&channels, None)?;

        // Mix to mono and convert back to i16
        let mono: Vec<i16> = if output.len() == 1 {
            output[0].iter().map(|&s| (s * 32767.0) as i16).collect()
        } else {
            let num_samples = output[0].len();
            (0..num_samples)
                .map(|i| {
                    let sum: f32 = output.iter().map(|ch| ch[i]).sum();
                    let avg = sum / output.len() as f32;
                    (avg * 32767.0) as i16
                })
                .collect()
        };

        Ok(mono)
    }

    /// Simple fast downsample for cases where quality isn't critical (e.g., level metering).
    pub fn downsample_simple(input: &[i16], source_rate: u32, channels: u16) -> Vec<i16> {
        let ratio = (source_rate / 16000) as usize;
        let step = (ratio * channels as usize).max(1);
        input.iter().step_by(step).copied().collect()
    }
}

/// Calculate RMS energy of a PCM buffer (for silence detection / level metering).
pub fn calculate_rms(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum / samples.len() as f64).sqrt() as f32
}

/// Normalized audio level (0.0 - 1.0) suitable for UI display.
pub fn audio_level(samples: &[i16]) -> f32 {
    let rms = calculate_rms(samples);
    (rms / 3000.0).min(1.0)
}
