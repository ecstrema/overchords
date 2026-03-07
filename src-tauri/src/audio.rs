use std::sync::{
    Arc, Mutex, atomic::{AtomicBool, AtomicI8, Ordering}
};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use spectrum_analyzer::{samples_fft_to_spectrum, scaling::scale_to_zero_to_one, FrequencyLimit};
use tauri::{AppHandle, Emitter};

use serde::Serialize;

static RUNNING: AtomicBool = AtomicBool::new(false);
static NOTES_TO_KEEP: AtomicI8 = AtomicI8::new(5);

#[derive(Serialize, Debug, Clone)]
pub struct NoteEvent {
    pub midi: u8,
    pub frequency: f32,
    pub magnitude: f32,
}

// simple frequency -> midi / name utilities
fn frequency_to_midi(freq: f32) -> f32 {
    if freq <= 0.0 {
        return 0.0;
    }
    let midi = 12.0 * (freq / 440.0).log2() + 69.0;
    midi.clamp(0.0, 127.0)
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
        // try to select an output device for loopback. fall back to default input.
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .or_else(|| host.default_input_device())
            .expect("no audio device available");

        // `SupportedStreamConfig` can be converted into a `StreamConfig`.
        let config: cpal::StreamConfig = if let Ok(input_cfg) = device.default_input_config() {
            input_cfg.into()
        } else if let Ok(output_cfg) = device.default_output_config() {
            output_cfg.into()
        } else {
            panic!("unable to obtain stream config");
        };

        // `SampleRate` is a u32 alias, not a tuple struct
        let sample_rate = config.sample_rate as f32;
        let channels = config.channels as usize;

        // buffer for mono samples
        let buffer = Arc::new(Mutex::new(Vec::with_capacity(16384)));
        let buf_clone = buffer.clone();

        // build input stream
        let err_fn = |e| eprintln!("audio stream error: {:?}", e);
        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // convert to mono
                    let mut buf = buf_clone.lock().unwrap();
                    for frame in data.chunks(channels) {
                        // average channels
                        let mut sum = 0.0;
                        for &s in frame {
                            sum += s;
                        }
                        buf.push(sum / channels as f32);
                    }
                },
                err_fn,
                None,
            )
            .expect("failed to build input stream");

        stream.play().expect("failed to play stream");

        // analysis loop
        while RUNNING.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(50));

            let mut audio_samples = Vec::new();
            {
                let mut buf = buffer.lock().unwrap();
                if buf.len() >= 4096 {
                    audio_samples.append(&mut buf.drain(..4096).collect());
                }
            }

            if audio_samples.is_empty() {
                continue;
            }

            // perform FFT
            if let Ok(spectrum) = samples_fft_to_spectrum(
                &audio_samples,
                sample_rate as u32,
                FrequencyLimit::Max(12e3),
                Some(&scale_to_zero_to_one),
            ) {
                let data = spectrum.data();
                // convert to midi notes
                let mut notes_vec: Vec<NoteEvent> = Vec::with_capacity(128);
                // init vec with frequency and midi, set magnitude to 0
                for i in 0..128 {
                    let freq = 440.0 * 2f32.powf((i as f32 - 69.0) / 12.0);
                    notes_vec.push(NoteEvent {
                        midi: i,
                        frequency: freq,
                        magnitude: 0.0,
                    });
                }
                // distribute magnitude to nearest midi notes with linear interpolation
                for (freq, mag) in data.iter() {
                    let midi = frequency_to_midi(freq.val()).clamp(0.0, 127.0);
                    let lower = midi.floor() as usize;
                    let upper = midi.ceil() as usize;
                    let upper_weight = midi - lower as f32;
                    let lower_weight = 1.0 - upper_weight;
                    notes_vec[lower].magnitude += mag.val() * lower_weight;
                    notes_vec[upper].magnitude += mag.val() * upper_weight;
                }

                // keep only the N notes with highest magnitude, where N is set by NOTES_TO_KEEP
                let n = NOTES_TO_KEEP.load(Ordering::SeqCst) as usize;
                notes_vec.sort_by(|a, b| b.magnitude.partial_cmp(&a.magnitude).unwrap());
                notes_vec.truncate(n);

                app_handle
                    .emit("notes", notes_vec)
                    .expect("failed to emit notes event");
            }

            //
        }

        // dropping stream stops it automatically
    });
}

/// Signal the capturing thread to stop. Returns immediately.
pub fn stop_listening() {
    RUNNING.store(false, Ordering::SeqCst);
}

pub fn set_notes_to_keep(n: i8) {
    NOTES_TO_KEEP.store(n, Ordering::SeqCst);
}

fn apply_hps(spectrum: &[f32], harmonics_to_check: usize) -> Vec<f32> {
    let mut hps_spectrum = spectrum.to_vec();
    let len = spectrum.len();

    // We multiply the original spectrum by downsampled versions of itself
    for i in 2..=harmonics_to_check {
        for j in 0..(len / i) {
            // Multiply the fundamental bin by its i-th harmonic bin
            hps_spectrum[j] *= spectrum[j * i];
        }

        // Zero out the end of the spectrum that we can't downsample anymore
        for j in (len / i)..len {
            hps_spectrum[j] = 0.0;
        }
    }

    // Squaring or taking the root can help with contrast
    hps_spectrum.iter().map(|&v| v.sqrt()).collect()
}
