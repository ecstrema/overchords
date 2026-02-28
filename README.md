# Overchords

Overchords shows the curently playing notes from the speaker output in a piano view.

The rust backend uses the cpal library to capture the audio output and perform a Fourier transform to determine the frequencies being played. The frequencies are then mapped to musical notes and sent to the frontend.

The frontend is a window that shows the currently playing notes in a piano view. The notes are highlighted as they are played, allowing you to see the chords being played in real-time.

## Implementation details

* **Rust backend** (`src-tauri` crate) captures audio using [`cpal`](https://crates.io/crates/cpal).  A background thread builds an input stream, converts samples to mono and accumulates them in a buffer.
* FFT analysis is performed with `spectrum-analyzer`.  Samples are grouped into 4096‑sample blocks, transformed to a `FrequencySpectrum`, and peaks above a threshold are translated into MIDI notes.
* Frequency‑to‑note mapping uses the standard 440 Hz tuning; notes are serialized and emitted as a `notes` event to the frontend using Tauri's event system.  Commands `start_audio_listening` / `stop_audio_listening` control the capture thread.
* **Svelte frontend** listens for `notes` events, maintains a list of currently active note names, and renders a simple two‑octave keyboard (`src/components/Piano.svelte`).  Keys corresponding to active notes are highlighted.

## Building and running

1. Install dependencies:
   ```bash
   cd overchords
   npm install    # frontend
   cd src-tauri   # rust backend
   cargo build
   ```
2. During development run:
   ```bash
   npm run dev       # start Vite server for frontend
   cd src-tauri
   cargo tauri dev   # launches the Tauri application window
   ```
3. On first launch the application will start capturing audio automatically.  You can also call the Tauri commands at runtime from the devtools console:
   ```js
   await window.__TAURI__.invoke('start_audio_listening');
   await window.__TAURI__.invoke('stop_audio_listening');
   ```

The UI will update in real time as notes are detected from the system audio.
