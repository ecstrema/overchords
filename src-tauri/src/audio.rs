use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use spectrum_analyzer::{samples_fft_to_spectrum, scaling::scale_to_zero_to_one, FrequencyLimit};
use tauri::{AppHandle, Emitter};

use serde::Serialize;

static RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Debug, Clone)]
pub struct NoteEvent {
    pub midi: u8,
    pub frequency: f32,
    pub magnitude: f32,
}

// simple frequency -> midi / name utilities
fn frequency_to_midi(freq: f32) -> u8 {
    if freq <= 0.0 {
        return 0;
    }
    let midi = 12.0 * (freq / 440.0).log2() + 69.0;
    midi.round().clamp(0.0, 127.0) as u8
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
                // smarter peak picking:
                // 1. compute a relative threshold based on the maximum magnitude
                // 2. only consider local maxima in the spectrum
                // 3. keep only the highest-valued bin for each midi note
                // 4. emit up to a fixed number of strongest notes
                let data = spectrum.data();
                let mut notes_map: std::collections::HashMap<u8, (f32, f32)> =
                    std::collections::HashMap::new();

                if !data.is_empty() {
                    // find max magnitude
                    let max_val = data.iter().map(|(_, v)| v.val()).fold(0.0, f32::max);
                    // relative threshold (20% of peak) but at least a small absolute floor
                    let rel_thresh = max_val * 0.7;
                    let abs_floor = 0.05;
                    let threshold = rel_thresh.max(abs_floor);

                    // scan for local maxima
                    for i in 1..data.len() - 1 {
                        let (freq, val) = data[i];
                        let mag = val.val();
                        if mag < threshold {
                            continue;
                        }
                        let prev = data[i - 1].1.val();
                        let next = data[i + 1].1.val();
                        if mag >= prev && mag >= next {
                            let midi = frequency_to_midi(freq.val());
                            let entry = notes_map.entry(midi).or_insert((mag, freq.val()));
                            if mag > entry.0 {
                                *entry = (mag, freq.val());
                            }
                        }
                    }
                }

                // convert hashmap to list including magnitude and sort by mag desc
                let mut notes_vec: Vec<(u8, f32, f32)> = notes_map
                    .into_iter()
                    .map(|(midi, (mag, freq))| (midi, freq, mag))
                    .collect();
                notes_vec
                    .sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

                let mut notes = Vec::new();
                for (midi, freq, mag) in notes_vec.iter().take(5) {
                    notes.push(NoteEvent {
                        midi: *midi,
                        frequency: *freq,
                        magnitude: *mag,
                    });
                }

                let _ = app_handle.emit("notes", notes.clone());
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
