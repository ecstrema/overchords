use ndarray::{s, Array2, ArrayView3};

#[cfg(any(target_os = "windows", target_os = "linux"))]
use ort::execution_providers::CUDAExecutionProvider;

#[cfg(target_os = "windows")]
use ort::execution_providers::DirectMLExecutionProvider;

#[cfg(target_os = "macos")]
use ort::execution_providers::CoreMLExecutionProvider;

use ort::{inputs, session::builder::GraphOptimizationLevel, session::Session};
use std::path::Path;

// Constants come from basic_pitch/constants.py on github
pub const AUDIO_SAMPLE_RATE: usize = 22050;
pub const FFT_HOP: usize = 256; // samples (11.6ms) - this is the hop size used by the Basic Pitch model, and determines the time resolution of the output annotations.
pub const AUDIO_WINDOW_LENGTH: usize = 2; // seconds
pub const AUDIO_TOTAL_SAMPLES: usize = AUDIO_SAMPLE_RATE * AUDIO_WINDOW_LENGTH;
pub const AUDIO_N_SAMPLES: usize = AUDIO_TOTAL_SAMPLES - FFT_HOP;

pub const DEFAULT_OVERLAPPING_FRAMES: usize = 30;
pub const OVERLAP_LEN: usize = DEFAULT_OVERLAPPING_FRAMES * FFT_HOP; // 7680

pub const N_OVERLAP_FRAMES_HALF: usize = DEFAULT_OVERLAPPING_FRAMES / 2; // 15

pub const NUM_CHANNELS: usize = 1; // Basic Pitch model expects mono audio input

pub struct ModelOutput {
    pub note: Array2<f32>,
    pub onset: Array2<f32>,
    pub contour: Array2<f32>,
    pub processed_sample_count: usize,
}

pub struct BasicPitchStreamer {
    session: Session,
    buffer: Vec<f32>,
}

impl BasicPitchStreamer {
    pub fn new<P: AsRef<Path>>(model_path: P) -> ort::Result<Self> {
        let mut providers = Vec::new();

        // Priority list for execution providers. ORT will attempt them in order and fall back to CPU if needed.
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        providers.push(CUDAExecutionProvider::default().build());
        #[cfg(target_os = "windows")]
        providers.push(DirectMLExecutionProvider::default().build());
        #[cfg(target_os = "macos")]
        providers.push(CoreMLExecutionProvider::default().build());

        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_execution_providers(providers)?
            .commit_from_file(model_path)?;

        // To ensure we get the correct output for the first window, we need to start with a buffer that has 15 frames of padding (3840 samples) at the beginning.
        let initial_padding = vec![0.0; OVERLAP_LEN / 2];

        Ok(Self {
            session,
            buffer: initial_padding,
        })
    }

    /// Push new samples into the buffer and process the latest sliding window.
    /// Returns a ModelOutput if the buffer is fully primed (has reached 43,844 samples).
    pub fn process_stream(&mut self, new_samples: &[f32]) -> ort::Result<Option<ModelOutput>> {
        self.buffer.extend_from_slice(new_samples);

        // If we haven't accumulated the first 2 seconds of audio yet, wait.
        if self.buffer.len() < AUDIO_N_SAMPLES {
            return Ok(None);
        }

        // If our buffer has grown larger than the required window,
        // drain the OLDEST samples from the front to maintain exactly AUDIO_N_SAMPLES.
        let processed_samples = if self.buffer.len() > AUDIO_N_SAMPLES {
            let excess = self.buffer.len() - AUDIO_N_SAMPLES;
            self.buffer.drain(..excess);
            excess
        } else {
            0
        };

        // The buffer is now exactly 43,844 samples. Run inference!
        let dimensions = [1usize, AUDIO_N_SAMPLES, 1];
        // Note: Using `self.buffer.clone()` here because Tensor::from_array takes ownership of the vec
        let tensor = ort::value::Tensor::from_array((dimensions, self.buffer.clone()))?;

        let outputs = self.session.run(inputs![tensor])?;

        let contour = Self::extract_and_trim(&outputs, "StatefulPartitionedCall:0")?;
        let note = Self::extract_and_trim(&outputs, "StatefulPartitionedCall:1")?;
        let onset = Self::extract_and_trim(&outputs, "StatefulPartitionedCall:2")?;

        // We DO NOT drain the buffer here!
        // We leave it full so the next tiny chunk of audio slides the window forward.

        Ok(Some(ModelOutput {
            note,
            onset,
            contour,
            processed_sample_count: processed_samples,
        }))
    }

    /// Helper to extract an output tensor, unwrap the batch dimension,
    /// and trim the overlapping frames from the time dimension.
    fn extract_and_trim(
        outputs: &ort::session::SessionOutputs,
        output_name: &str,
    ) -> ort::Result<Array2<f32>> {
        // Extract as [1, time, freq]
        let (shape, data) = outputs[output_name].try_extract_tensor::<f32>()?;

        //Cast the shape dimensions to usize (ort usually returns i64)
        let batch = shape[0] as usize;
        let time = shape[1] as usize;
        let freq = shape[2] as usize;

        // Wrap the flat data slice into a 3D view
        let array_view = ArrayView3::from_shape((batch, time, freq), data)
            .expect("Data length does not match the expected [batch, time, freq] shape");

        // Trim the overlapping frames (15 frames from start, 15 from end)
        // This corresponds to Python's `output[:, n_olap:-n_olap, :]`
        let trimmed = array_view.slice(s![
            0,
            N_OVERLAP_FRAMES_HALF..time - N_OVERLAP_FRAMES_HALF,
            ..
        ]);

        // Convert the trimmed view into an owned 2D array [time, freq]
        Ok(trimmed.to_owned())
    }
}
