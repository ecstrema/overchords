use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit, scaling::scale_to_zero_to_one};
use tauri::{AppHandle, Emitter};

use serde::Serialize;

static RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Debug, Clone)]
pub struct NoteEvent {
    pub midi: u8,
    pub name: String,
    pub frequency: f32,
}

// simple frequency -> midi / name utilities
fn frequency_to_midi(freq: f32) -> u8 {
    if freq <= 0.0 {
        return 0;
    }
    let midi = 12.0 * (freq / 440.0).log2() + 69.0;
    let midi = midi.round();
    if midi < 0.0 {
        0
    } else if midi > 127.0 {
        127
    } else {
        midi as u8
    }
}

fn midi_to_name(midi: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let note = NAMES[(midi % 12) as usize];
    let octave = (midi / 12).saturating_sub(1); // midi 0 is C-1
    format!("{}{}", note, octave)
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
                FrequencyLimit::All,
                Some(&scale_to_zero_to_one),
            ) {
                // pick peaks above threshold
                let threshold = 0.1;
                let mut notes = Vec::new();
                for (freq, value) in spectrum.data() {
                    let val = value.val();
                    if val > threshold {
                        let midi = frequency_to_midi(freq.val());
                        if notes.iter().any(|n: &NoteEvent| n.midi == midi) {
                            continue;
                        }
                        notes.push(NoteEvent {
                            midi,
                            name: midi_to_name(midi),
                            frequency: freq.val(),
                        });
                    }
                }
                if !notes.is_empty() {
                    let _ = app_handle.emit("notes", notes.clone());
                }
            }
        }

        // dropping stream stops it automatically
    });
}

/// Signal the capturing thread to stop. Returns immediately.
pub fn stop_listening() {
    RUNNING.store(false, Ordering::SeqCst);
}
