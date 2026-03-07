use std::sync::{
    Arc, Mutex, atomic::{AtomicBool, AtomicI8, Ordering}
};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cqt_rs::{CQTParams, Cqt};
use tauri::{AppHandle, Emitter};

use serde::Serialize;

static RUNNING: AtomicBool = AtomicBool::new(false);
static NOTES_TO_KEEP: AtomicI8 = AtomicI8::new(5);

// ── CQT configuration ─────────────────────────────────────────────────────────
/// Lowest analysed frequency: C-1 (MIDI 0). 440 × 2^(−69/12) ≈ 8.176 Hz.
const CQT_MIN_FREQ: f32 = 8.176;
/// Highest analysed frequency: roughly C8 (MIDI 108).
const CQT_MAX_FREQ: f32 = 4186.0;
/// One bin per semitone → direct MIDI mapping.
const BINS_PER_OCTAVE: usize = 12;
/// Analysis window length (power-of-two, ≥ hop size).
const WINDOW_LENGTH: usize = 4096;
/// Hop between successive CQT frames inside one analysis chunk.
const HOP_SIZE: usize = 512;
/// MIDI note that corresponds to CQT bin 0 (C-1).
const MIDI_OFFSET: u8 = 0;
/// Notes below this fraction of the peak HPS magnitude are excluded.
/// Keeps multiple notes when they are comparably strong, while suppressing noise.
const RELATIVE_THRESHOLD: f32 = 0.15;

#[derive(Serialize, Debug, Clone)]
pub struct NoteEvent {
    pub midi: u8,
    pub frequency: f32,
    pub magnitude: f32,
}

/// Start capturing audio in a background thread and emit `notes` events on the provided AppHandle.
/// If a listener is already running this will do nothing.
pub fn start_listening(app_handle: AppHandle) {
    if RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        // already running
        return;
    }

    thread::spawn(move || {
        // Try to select an output device for loopback, fall back to default input.
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .or_else(|| host.default_input_device())
            .expect("no audio device available");

        let config: cpal::StreamConfig = if let Ok(input_cfg) = device.default_input_config() {
            input_cfg.into()
        } else if let Ok(output_cfg) = device.default_output_config() {
            output_cfg.into()
        } else {
            panic!("unable to obtain stream config");
        };

        let sample_rate = config.sample_rate as usize;
        let channels = config.channels as usize;

        // Build the CQT analyser once — this pre-computes the filterbank (~3 ms).
        let cqt_params = CQTParams::new(
            CQT_MIN_FREQ,
            CQT_MAX_FREQ,
            BINS_PER_OCTAVE,
            sample_rate,
            WINDOW_LENGTH,
        )
        .expect("failed to create CQT params");
        let num_bins = cqt_params.num_bins();
        let cqt = Cqt::new(cqt_params);

        // Ring-buffer for mono samples.
        let buffer = Arc::new(Mutex::new(Vec::<f32>::with_capacity(16384)));
        let buf_clone = buffer.clone();

        let err_fn = |e| eprintln!("audio stream error: {:?}", e);
        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut buf = buf_clone.lock().unwrap();
                    for frame in data.chunks(channels) {
                        let sum: f32 = frame.iter().sum();
                        buf.push(sum / channels as f32);
                    }
                },
                err_fn,
                None,
            )
            .expect("failed to build input stream");

        stream.play().expect("failed to play stream");

        // ── Analysis loop ──────────────────────────────────────────────────────
        while RUNNING.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(50));

            // Drain a HOP_SIZE-aligned chunk once we have at least WINDOW_LENGTH samples.
            let audio_samples: Vec<f32> = {
                let mut buf = buffer.lock().unwrap();
                if buf.len() >= WINDOW_LENGTH {
                    let n = (buf.len() / HOP_SIZE) * HOP_SIZE;
                    buf.drain(..n).collect()
                } else {
                    Vec::new()
                }
            };

            if audio_samples.is_empty() {
                continue;
            }

            // Run CQT → Array2<f32> with shape (num_frames, num_bins).
            let cqt_matrix = match cqt.process(&audio_samples, HOP_SIZE) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("CQT error: {:?}", e);
                    continue;
                }
            };

            let (num_frames, _) = cqt_matrix.dim();
            if num_frames == 0 {
                continue;
            }

            // Average magnitudes across frames.
            let mut bin_magnitudes = vec![0.0f32; num_bins];
            for frame_idx in 0..num_frames {
                let row = cqt_matrix.row(frame_idx);
                for (bin, &mag) in row.iter().enumerate() {
                    if bin < num_bins {
                        bin_magnitudes[bin] += mag;
                    }
                }
            }
            for v in bin_magnitudes.iter_mut() {
                *v /= num_frames as f32;
            }

            // Apply HPS in the log-frequency (semitone) domain.
            let hps_mags = apply_hps_cqt(&bin_magnitudes, 3);

            // Normalise to [0, 1] relative to the peak bin so that multiple
            // simultaneously active notes all score proportionally rather than
            // being crushed by the absolute scale of the magnitudes.
            let peak = hps_mags.iter().fold(0.0f32, |a, &b| a.max(b));
            if peak == 0.0 {
                continue;
            }
            let hps_norm: Vec<f32> = hps_mags.iter().map(|&v| v / peak).collect();

            // Build note events for bins above the relative threshold, then
            // keep only the top-N strongest ones.
            let mut notes_vec: Vec<NoteEvent> = hps_norm
                .iter()
                .enumerate()
                .filter(|&(_, &mag)| mag >= RELATIVE_THRESHOLD)
                .map(|(bin, &magnitude)| {
                    let midi = MIDI_OFFSET.saturating_add(bin as u8);
                    let frequency =
                        CQT_MIN_FREQ * 2f32.powf(bin as f32 / BINS_PER_OCTAVE as f32);
                    NoteEvent { midi, frequency, magnitude }
                })
                .collect();

            let n = NOTES_TO_KEEP.load(Ordering::SeqCst) as usize;
            notes_vec.sort_by(|a, b| b.magnitude.partial_cmp(&a.magnitude).unwrap());
            notes_vec.truncate(n);

            app_handle
                .emit("notes", notes_vec)
                .expect("failed to emit notes event");
        }

        // dropping `stream` stops capture automatically
    });
}

/// Signal the capturing thread to stop. Returns immediately.
pub fn stop_listening() {
    RUNNING.store(false, Ordering::SeqCst);
}

pub fn set_notes_to_keep(n: i8) {
    NOTES_TO_KEEP.store(n, Ordering::SeqCst);
}

/// Apply Harmonic Product Spectrum in CQT (log-frequency / semitone) space.
///
/// In a semitone-spaced CQT the k-th harmonic of fundamental bin `b` falls at:
///   `b + round(12 · log₂(k))`
///
/// Precomputed offsets (for k = 2 … 5):
/// | k | offset |
/// |---|--------|
/// | 2 |   12   | one octave
/// | 3 |   19   | octave + perfect fifth
/// | 4 |   24   | two octaves
/// | 5 |   28   | two octaves + major third
/// | 6 |   31   | two octaves + perfect fifth
/// | 7 |   34   | two octaves + minor seventh
/// | 8 |   36   | three octaves
///
/// `harmonics` controls how many of these are used (capped at 7).
fn apply_hps_cqt(magnitudes: &[f32], harmonics: usize) -> Vec<f32> {
    const HARMONIC_OFFSETS: [usize; 7] = [12, 19, 24, 28, 31, 34, 36];
    let len = magnitudes.len();
    let mut hps = magnitudes.to_vec();

    for &offset in HARMONIC_OFFSETS.iter().take(harmonics) {
        for b in 0..len {
            let harmonic_bin = b + offset;
            if harmonic_bin < len {
                hps[b] *= magnitudes[harmonic_bin];
            } else {
                hps[b] = 0.0;
            }
        }
    }

    // sqrt to partially undo the effect of repeated multiplication
    hps.iter().map(|&v| v.sqrt()).collect()
}
