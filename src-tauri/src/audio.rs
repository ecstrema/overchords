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
    BasicPitchStreamer, AUDIO_N_SAMPLES, AUDIO_SAMPLE_RATE, FFT_HOP, NUM_CHANNELS,
};

static RUNNING: AtomicBool = AtomicBool::new(false);
static NOTES_TO_KEEP: AtomicUsize = AtomicUsize::new(5);
static NOTES_PROBABILITY_THRESHOLD: AtomicU8 = AtomicU8::new(128); // Default threshold is 0.5 when scaled to [0, 255]
static FRAMES_TO_CHECK: AtomicUsize = AtomicUsize::new(15); // Check the last 15 frames (0.087s) for note onsets, which is where they will be most accurate.
pub fn set_notes_to_keep(n: usize) {
    NOTES_TO_KEEP.store(n, Ordering::SeqCst);
}

pub fn set_note_probability_threshold(threshold: f32) {
    NOTES_PROBABILITY_THRESHOLD.store((threshold * 255.0) as u8, Ordering::SeqCst);
}

pub fn set_frames_to_check(n: usize) {
    println!("Setting frames to check to {}", n);
    FRAMES_TO_CHECK.store(n, Ordering::SeqCst);
}

const MIDI_OFFSET: u8 = 21; // Basic Pitch outputs 88 bins starting at A0 (MIDI 21)

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

        let chunk_size = if let Some(r) = &resampler {
            r.input_frames_next()
        } else {
            1024
        };
        let mut audio_buffer = Vec::with_capacity(chunk_size * 10);
        let mut resampled = vec![0.0; chunk_size * 10 * 2]; // Give plenty of room

        // Inference Loop
        while RUNNING.load(Ordering::SeqCst) {
            let start_time = std::time::Instant::now();

            // Drain ONLY exact multiples of the chunk size.
            // Any leftovers safely remain in the FIFO for the next loop.
            if !drain_audio_fifo(&raw_audio_fifo, &mut audio_buffer, chunk_size) {
                continue;
            }

            // Resample (or just use raw)
            let audio_to_process = if let Some(resampler_ref) = resampler.as_mut() {
                let written = resample_audio_buffer(&audio_buffer, &mut resampled, resampler_ref);
                &resampled[..written]
            } else {
                &audio_buffer[..]
            };

            // Process the continuous stream
            // (BasicPitchStreamer buffers the 43,844 window internally)
            // Process the continuous stream
            let model_output = model.process_stream(&audio_to_process).unwrap_or_else(|e| {
                eprintln!("Error during model inference: {:?}", e);
                None
            });

            if let Some(output) = model_output {
                process_model_outputs(&app_handle, vec![output]);
            }

            // Maintain ~30Hz loop cadence
            let elapsed = start_time.elapsed();
            if elapsed.as_millis() > 100 {
                eprintln!(
                    "Warning: Inference loop is taking too long ({} ms)",
                    elapsed.as_millis()
                );
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

    let mut frames_to_check = FRAMES_TO_CHECK.load(Ordering::SeqCst);
    if frames_to_check == 0 {
        // auto mode
        // calculate the number of frames from the model output's processed samples
        frames_to_check = (output.processed_sample_count / FFT_HOP).max(1).min(100);
    }
    let frames_to_skip = output.note.shape()[0].saturating_sub(frames_to_check);

    for (i, col) in output.note.columns().into_iter().enumerate() {
        // Find the max probability in the trailing edge of the window
        let max_prob = col.iter().skip(frames_to_skip).cloned().fold(0.0, f32::max);

        if max_prob > threshold {
            note_events.push(NoteEvent {
                midi: (i as u8) + MIDI_OFFSET,
                probability: max_prob,
            });
        }
    }

    // Keep only the top N notes by probability
    if note_events.len() > NOTES_TO_KEEP.load(Ordering::SeqCst) {
        note_events.sort_by(|a, b| b.probability.partial_cmp(&a.probability).unwrap());
        note_events.truncate(NOTES_TO_KEEP.load(Ordering::SeqCst));
    }
    note_events
}

fn resample_audio_buffer(
    audio_buffer: &Vec<f32>,
    resampled: &mut Vec<f32>,
    resampler: &mut Fft<f32>,
) -> usize {
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
    let mut input_frames_next = resampler.input_frames_next();

    while input_frames_left >= input_frames_next {
        let (frames_read, frames_written) = resampler
            .process_into_buffer(&input_adapter, &mut output_adapter, Some(&indexing))
            .expect("Failed to resample audio");

        indexing.input_offset += frames_read;
        indexing.output_offset += frames_written;
        input_frames_left -= frames_read;
        input_frames_next = resampler.input_frames_next();
    }

    indexing.output_offset
}

/// Drains the raw audio fifo samples into the audio_buffer.
/// Returns true if enough samples were availabe, false if not.
fn drain_audio_fifo(
    raw_audio_fifo: &Arc<Mutex<Vec<f32>>>,
    audio_buffer: &mut Vec<f32>,
    chunk_size: usize,
) -> bool {
    let mut fifo = match raw_audio_fifo.lock() {
        Ok(g) => g,
        Err(err) => err.into_inner(),
    };

    // if we don't have enough samples for a single window, we do not drain.
    let chunks_available = fifo.len() / chunk_size;
    if chunks_available == 0 {
        return false; // Not enough for a full chunk yet
    }

    let samples_to_take = chunks_available * chunk_size;
    audio_buffer.clear();
    audio_buffer.extend(fifo.drain(..samples_to_take));

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
