# Overchords

Overchords shows the curently playing notes from the speaker output in a piano view.

The rust backend uses the cpal library to capture the audio output and perform a Fourier transform to determine the frequencies being played. The frequencies are then mapped to musical notes and sent to the frontend.

The frontend is a window that shows the currently playing notes in a piano view. The notes are highlighted as they are played, allowing you to see the chords being played in real-time.

# Development

First, make sure you have Rust and Cargo installed. You can install them from [rustup.rs](https://rustup.rs/).

Also, you'll need [Bun](https://bun.sh/).

Then, install the dependencies with `bun install`.

Finally, you can run the application with `bun tauri dev`.
