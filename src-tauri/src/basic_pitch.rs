use ndarray::{s, Array2, ArrayView3};
use ort::value::Tensor;
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
pub const HOP_SIZE: usize = AUDIO_N_SAMPLES - OVERLAP_LEN; // 36164

pub const N_OVERLAP_FRAMES_HALF: usize = DEFAULT_OVERLAPPING_FRAMES / 2; // 15

pub const NUM_CHANNELS: usize = 1; // Basic Pitch model expects mono audio input

pub struct ModelOutput {
    pub note: Array2<f32>,
    pub onset: Array2<f32>,
    pub contour: Array2<f32>,
}

pub struct BasicPitchStreamer {
    session: Session,
    buffer: Vec<f32>,
}

impl BasicPitchStreamer {
    pub fn new<P: AsRef<Path>>(model_path: P) -> ort::Result<Self> {
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .commit_from_file(model_path)?;

        // To ensure we get the correct output for the first window, we need to start with a buffer that has 15 frames of padding (3840 samples) at the beginning.
        let initial_padding = vec![0.0; OVERLAP_LEN / 2];

        Ok(Self {
            session,
            buffer: initial_padding,
        })
    }

    /// Push new samples into the buffer and process as many windows as possible.
    /// Returns a vector of ModelOutputs. This vector will be empty if not enough samples have been buffered to complete a window.
    pub fn process_stream(&mut self, new_samples: &[f32]) -> ort::Result<Vec<ModelOutput>> {
        self.buffer.extend_from_slice(new_samples);
        let mut results = Vec::new();

        // While we have enough samples for a full model input window
        while self.buffer.len() >= AUDIO_N_SAMPLES {
            // Take the first AUDIO_N_SAMPLES from the buffer as input
            let buffer = &self.buffer[..AUDIO_N_SAMPLES];
            // Shape it as [batch, samples, channels] for the model
            let dimensions = [1usize, AUDIO_N_SAMPLES, 1];
            let tensor = Tensor::from_array((dimensions, buffer.to_vec()))?;

            let outputs = self.session.run(inputs![tensor])?;

            let contour = Self::extract_and_trim(&outputs, "StatefulPartitionedCall:0")?;
            let note = Self::extract_and_trim(&outputs, "StatefulPartitionedCall:1")?;
            let onset = Self::extract_and_trim(&outputs, "StatefulPartitionedCall:2")?;

            results.push(ModelOutput {
                note,
                onset,
                contour,
            });

            // Advance the buffer by HOP_SIZE (36,164 samples)
            // This leaves exactly OVERLAP_LEN (7,680 samples) as the prefix for the next window.
            self.buffer.drain(..HOP_SIZE);
        }

        Ok(results)
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
