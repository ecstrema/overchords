mod audio;
use audio::{start_listening, stop_listening};
mod basic_pitch;

/// start capturing audio output and emit `notes` events
#[tauri::command]
fn start_audio_listening(app: tauri::AppHandle) {
    // spin up background thread to perform FFT and emit events
    start_listening(app);
}

/// stop the audio capture thread
#[tauri::command]
fn stop_audio_listening() {
    stop_listening();
}

#[tauri::command]
fn set_notes_to_keep(n: usize) {
    audio::set_notes_to_keep(n);
}

#[tauri::command]
fn set_note_probability_threshold(threshold: f32) {
    audio::set_note_probability_threshold(threshold);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    return tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            start_audio_listening,
            stop_audio_listening,
            set_notes_to_keep,
            set_note_probability_threshold,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
