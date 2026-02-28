// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

mod audio;
use audio::{start_listening, stop_listening};

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            start_audio_listening,
            stop_audio_listening,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
