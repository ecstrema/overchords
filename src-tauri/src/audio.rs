use std::sync::{
    atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::thread;

use audioadapter_buffers::direct::InterleavedSlice;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rubato::{Fft, FixedSync, Indexing, Resampler};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::basic_pitch::{
    BasicPitchStreamer, AUDIO_N_SAMPLES, AUDIO_SAMPLE_RATE, AUDIO_TOTAL_SAMPLES, FFT_HOP,
    NUM_CHANNELS,
};

static RUNNING: AtomicBool = AtomicBool::new(false);
static NOTES_TO_KEEP: AtomicUsize = AtomicUsize::new(5);
static NOTES_PROBABILITY_THRESHOLD: AtomicU8 = AtomicU8::new((THRESHOLD * 255.0) as u8);

pub fn set_notes_to_keep(n: usize) {
    NOTES_TO_KEEP.store(n, Ordering::SeqCst);
}

pub fn set_note_probability_threshold(threshold: f32) {
    NOTES_PROBABILITY_THRESHOLD.store((threshold * 255.0) as u8, Ordering::SeqCst);
}

const MIDI_OFFSET: u8 = 21; // Basic Pitch outputs 88 bins starting at A0 (MIDI 21)
const THRESHOLD: f32 = 0.5;

#[derive(Serialize, Debug, Clone)]
pub struct NoteEvent {
    pub midi: u8,
    pub probability: f32,
}

pub fn start_listening(app_handle: AppHandle) {
    if RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    thread::spawn(move || {
        let mut model =
            BasicPitchStreamer::new(get_model_path(&app_handle)).expect("Failed to load model");

        // Keep stream alive by having it returned by the function. Else it is dropped.
        let (raw_audio_fifo, source_sample_rate, _stream) =
            start_listening_to_computer_audio(AUDIO_N_SAMPLES * 2);

        let resampling_ratio = AUDIO_SAMPLE_RATE as f32 / source_sample_rate as f32;
        let samples_per_window = (AUDIO_N_SAMPLES as f32 / resampling_ratio).ceil() as usize;

        let mut resampler = if source_sample_rate as usize != AUDIO_SAMPLE_RATE {
            Some(
                Fft::<f32>::new(
                    source_sample_rate as usize,
                    AUDIO_SAMPLE_RATE,
                    1024,
                    1,
                    1,
                    FixedSync::Input,
                )
                .expect("Failed to create resampler"),
            )
        } else {
            None
        };

        let mut audio_buffer = Vec::with_capacity(samples_per_window * 2);
        let mut resampled = vec![0.0; AUDIO_TOTAL_SAMPLES + 2048]; // Add some extra padding to ensure we can always resample a full window even if the input buffer is slightly underfilled. The resampler will just write zeros for the missing input samples.

        // Inference Loop
        while RUNNING.load(Ordering::SeqCst) {
            let start_time = std::time::Instant::now();

            // Drain the FIFO into a local buffer for processing
            if !drain_audio_fifo(&raw_audio_fifo, &mut audio_buffer, samples_per_window) {
                continue;
            }

            let audio_to_process = if let Some(resampler_ref) = resampler.as_mut() {
                resample_audio_buffer(&audio_buffer, &mut resampled, resampler_ref);
                &resampled[..AUDIO_N_SAMPLES] // Slice away the padding!
            } else {
                &audio_buffer[..AUDIO_N_SAMPLES] // No resampler needed, use raw data
            };

            // Process the audio buffer through the model
            let model_outputs = model.process_stream(&audio_to_process).unwrap_or_else(|e| {
                eprintln!("Error during model inference: {:?}", e);
                Vec::new()
            });

            process_model_outputs(&app_handle, model_outputs);

            // Maintain ~30Hz loop cadence
            let elapsed = start_time.elapsed();
            if elapsed.as_millis() > 100 {
                eprintln!("Warning: Inference loop is taking too long ({} ms)", elapsed.as_millis());
            }
            let sleep_dur = std::time::Duration::from_millis(30).saturating_sub(elapsed);
            std::thread::sleep(sleep_dur);
        }
    });
}

/// Convert model outputs to NoteEvents and emit to frontend
fn process_model_outputs(
    app_handle: &AppHandle,
    model_outputs: Vec<crate::basic_pitch::ModelOutput>,
) {
    for output in model_outputs {
        let note_events = extract_note_events(output);

        if !note_events.is_empty() {
            app_handle
                .emit("notes", note_events)
                .unwrap_or_else(|e| eprintln!("Failed to emit notes event: {:?}", e));
        }
    }
}

fn extract_note_events(output: crate::basic_pitch::ModelOutput) -> Vec<NoteEvent> {
    let mut note_events = Vec::new();
    let threshold = NOTES_PROBABILITY_THRESHOLD.load(Ordering::SeqCst) as f32 / 255.0;
    // Iterate over the columns (the 88 frequency bins)
    for (i, col) in output.note.columns().into_iter().enumerate() {
        // Find the highest probability for this specific pitch in the current time window
        let max_prob = col.into_iter().cloned().fold(0.0, f32::max);

        if max_prob > threshold {
            note_events.push(NoteEvent {
                midi: (i as u8) + MIDI_OFFSET,
                probability: max_prob,
            });
        }
    }
    // Keep only the top N notes by probability
    note_events.sort_by(|a, b| b.probability.partial_cmp(&a.probability).unwrap());
    note_events.truncate(NOTES_TO_KEEP.load(Ordering::SeqCst));
    note_events
}

fn resample_audio_buffer(
    audio_buffer: &Vec<f32>,
    resampled: &mut Vec<f32>,
    resampler_ref: &mut Fft<f32>,
) {
    let input_adapter =
        InterleavedSlice::new(audio_buffer, NUM_CHANNELS, audio_buffer.len()).unwrap();

    let resampled_len = resampled.len();
    let mut output_adapter =
        InterleavedSlice::new_mut(resampled, NUM_CHANNELS, resampled_len).unwrap();

    let mut indexing = Indexing {
        input_offset: 0,
        output_offset: 0,
        active_channels_mask: None,
        partial_len: None,
    };

    let mut input_frames_left = audio_buffer.len();
    let mut input_frames_next = resampler_ref.input_frames_next();

    while input_frames_left >= input_frames_next {
        let (frames_read, frames_written) = resampler_ref
            .process_into_buffer(&input_adapter, &mut output_adapter, Some(&indexing))
            .expect("Failed to resample audio");

        indexing.input_offset += frames_read;
        indexing.output_offset += frames_written;
        input_frames_left -= frames_read;
        input_frames_next = resampler_ref.input_frames_next();
    }
}

/// Drains the raw audio fifo samples into the audio_buffer.
/// Returns true if enough samples were availabe, false if not.
fn drain_audio_fifo(
    raw_audio_fifo: &Arc<Mutex<Vec<f32>>>,
    audio_buffer: &mut Vec<f32>,
    samples_per_window: usize,
) -> bool {
    let mut fifo = match raw_audio_fifo.lock() {
        Ok(g) => g,
        Err(err) => {
            eprintln!(
                "Warning: raw_audio_fifo mutex poisoned in inference loop: {:?}. Recovering.",
                err
            );
            err.into_inner()
        }
    };
    let fifo_len = fifo.len();

    // if we don't have enough samples for a single window, we do not drain.
    if fifo.len() < samples_per_window {
        return false;
    }

    // put the next window of samples into the audio buffer for processing. Remove the first FFT_HOP samples from the fifo to maintain the overlap for the next window.
    audio_buffer.clear();
    audio_buffer.extend_from_slice(&fifo[..samples_per_window]);

    // If there is more than 1 FFT_HOP samples more than the current_window, they will all be drained, keeping a single FFT_HOP of overlap for the next window.
    fifo.drain(..fifo_len.saturating_sub(samples_per_window + FFT_HOP));

    return true;
}

/// Starts listening to the computer's audio output and pushes mono samples into the provided FIFO.
/// Returns the sample rate of the audio stream
fn start_listening_to_computer_audio(
    capacity_hint: usize,
) -> (Arc<Mutex<Vec<f32>>>, u32, cpal::Stream) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no audio device available");
    let config: cpal::StreamConfig = device.default_output_config().unwrap().into();

    let raw_audio_fifo = Arc::new(Mutex::new(Vec::<f32>::with_capacity(capacity_hint)));

    let stream_audio_fifo = raw_audio_fifo.clone();

    let stream = device
        .build_input_stream(
            &config,
            move |data, _| {
                let mut buf = match stream_audio_fifo.lock() {
                    Ok(g) => g,
                    Err(err) => {
                        eprintln!("Warning: stream_audio_fifo mutex poisoned in input callback: {:?}. Recovering.", err);
                        err.into_inner()
                    }
                };
                for frame in data.chunks(config.channels as usize) {
                    let mono = frame.iter().sum::<f32>() / config.channels as f32;
                    buf.push(mono);
                }
            },
            |e| eprintln!("Stream error: {:?}", e),
            None,
        )
        .expect("failed to build stream");

    stream.play().expect("failed to play stream");

    return (raw_audio_fifo, config.sample_rate, stream);
}

fn get_model_path(app_handle: &AppHandle) -> std::path::PathBuf {
    return app_handle
        .path()
        .resolve(
            "models/basic_pitch.onnx",
            tauri::path::BaseDirectory::Resource,
        )
        .expect("Failed to resolve resource path");
}

pub fn stop_listening() {
    RUNNING.store(false, Ordering::SeqCst);
}
