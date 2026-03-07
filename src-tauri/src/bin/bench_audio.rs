/// Quick timing benchmark to compare CQT vs plain FFT under our exact audio.rs parameters.
///
/// Run with:
///   cargo run --bin bench_audio                        (debug)
///   cargo run --bin bench_audio --release              (release/optimised)
///   cargo run --bin bench_audio --profile bench-fast   (opt-level 3, like a dev dep)

use std::time::{Duration, Instant};
use cqt_rs::{CQTParams, Cqt};
use rustfft::{FftPlanner, num_complex::Complex};

// ── parameters copied verbatim from audio.rs ──────────────────────────────────
const CQT_MIN_FREQ: f32 = 8.176;
const CQT_MAX_FREQ: f32 = 4186.0;
const BINS_PER_OCTAVE: usize = 12;
const WINDOW_LENGTH: usize = 4096;
const HOP_SIZE: usize = 512;
const SAMPLE_RATE: usize = 44100;

/// Synthesise a chord of sine waves (A4 + E5 + C5).
fn make_signal(samples: usize) -> Vec<f32> {
    let freqs = [261.63_f32, 329.63, 392.00, 440.0]; // C4 E4 G4 A4
    (0..samples)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            freqs.iter().map(|&f| (2.0 * std::f32::consts::PI * f * t).sin()).sum::<f32>()
                / freqs.len() as f32
        })
        .collect()
}

/// Simple repeated-call timer. Returns (mean, min, max) across `iters` calls.
fn time_fn<F: FnMut()>(mut f: F, iters: u32) -> (Duration, Duration, Duration) {
    let mut times = Vec::with_capacity(iters as usize);
    for _ in 0..iters {
        let t0 = Instant::now();
        f();
        times.push(t0.elapsed());
    }
    let mean = times.iter().sum::<Duration>() / iters;
    let min = *times.iter().min().unwrap();
    let max = *times.iter().max().unwrap();
    (mean, min, max)
}

fn main() {
    // Amount of audio accumulated over one 50 ms tick at 44100 Hz.
    let tick_samples = (SAMPLE_RATE as f32 * 0.050).round() as usize; // ≈ 2205
    // We drain when buf >= WINDOW_LENGTH; after two ticks buf ≈ 4410 → drain 4096.
    let chunk_samples = (WINDOW_LENGTH / HOP_SIZE) * HOP_SIZE; // exact multiple
    let signal = make_signal(chunk_samples);
    let num_frames = chunk_samples / HOP_SIZE;

    println!("=== Audio analysis benchmark ===");
    println!("  SAMPLE_RATE    : {} Hz", SAMPLE_RATE);
    println!("  WINDOW_LENGTH  : {} samples", WINDOW_LENGTH);
    println!("  HOP_SIZE       : {} samples", HOP_SIZE);
    println!("  chunk_samples  : {} ({} frames/chunk)", chunk_samples, num_frames);
    println!("  tick_samples   : {} (~50 ms)", tick_samples);
    println!();

    // ── 1. CQT filterbank creation ────────────────────────────────────────────
    let (cqt_build_mean, cqt_build_min, _) = time_fn(|| {
        let params = CQTParams::new(
            CQT_MIN_FREQ, CQT_MAX_FREQ, BINS_PER_OCTAVE, SAMPLE_RATE, WINDOW_LENGTH,
        ).unwrap();
        let _cqt = Cqt::new(params); // builds filterbank
    }, 5);
    println!("Cqt::new (filterbank build, 5 runs)");
    println!("  mean: {:>8.2?}  min: {:>8.2?}", cqt_build_mean, cqt_build_min);

    // Build once (this is what audio.rs does).
    let params = CQTParams::new(
        CQT_MIN_FREQ, CQT_MAX_FREQ, BINS_PER_OCTAVE, SAMPLE_RATE, WINDOW_LENGTH,
    ).unwrap();
    let num_bins = params.num_bins();
    let cqt = Cqt::new(params);
    println!("  num_bins: {}", num_bins);
    println!();

    // ── 2. CQT process — warm-up then timed ───────────────────────────────────
    cqt.process(&signal, HOP_SIZE).unwrap(); // warm-up
    let (cqt_mean, cqt_min, cqt_max) = time_fn(|| {
        let _ = cqt.process(&signal, HOP_SIZE).unwrap();
    }, 30);
    println!("cqt.process ({} frames, 30 runs)", num_frames);
    println!("  mean: {:>8.2?}  min: {:>8.2?}  max: {:>8.2?}", cqt_mean, cqt_min, cqt_max);
    println!("  budget: 50 ms tick / {:>5.2?} = {:.1}× headroom",
        cqt_mean, Duration::from_millis(50).as_secs_f64() / cqt_mean.as_secs_f64());
    println!();

    // ── 3. Plain FFT (what spectrum-analyzer used) ────────────────────────────
    // One forward FFT on WINDOW_LENGTH complex samples — this is the spectrum_analyzer cost.
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(WINDOW_LENGTH);
    let mut buf: Vec<Complex<f32>> = signal.iter().map(|&s| Complex { re: s, im: 0.0 }).collect();
    fft.process(&mut buf); // warm-up
    let (fft_mean, fft_min, fft_max) = time_fn(|| {
        let mut b: Vec<Complex<f32>> =
            signal.iter().map(|&s| Complex { re: s, im: 0.0 }).collect();
        fft.process(&mut b);
    }, 30);
    println!("Plain FFT ({} samples, reusing planner, 30 runs)", WINDOW_LENGTH);
    println!("  mean: {:>8.2?}  min: {:>8.2?}  max: {:>8.2?}", fft_mean, fft_min, fft_max);
    println!();

    // ── 4. FFT with fresh planner each call (spectrum-analyzer pattern) ───────
    let (fft_fresh_mean, fft_fresh_min, fft_fresh_max) = time_fn(|| {
        let mut p = FftPlanner::<f32>::new();
        let f = p.plan_fft_forward(WINDOW_LENGTH);
        let mut b: Vec<Complex<f32>> =
            signal.iter().map(|&s| Complex { re: s, im: 0.0 }).collect();
        f.process(&mut b);
    }, 30);
    println!("Plain FFT with fresh planner each call (30 runs)");
    println!("  mean: {:>8.2?}  min: {:>8.2?}  max: {:>8.2?}", fft_fresh_mean, fft_fresh_min, fft_fresh_max);
    println!();

    // ── 5. Overhead ratio ─────────────────────────────────────────────────────
    println!("=== Summary ===");
    let ratio = cqt_mean.as_secs_f64() / fft_mean.as_secs_f64();
    println!("  CQT / plain-FFT ratio : {:.1}×  (CQT does {} filterbank dot-products)", ratio, num_frames);
    println!("  CQT is {}",
        if cqt_mean < Duration::from_millis(50) { "WITHIN 50 ms budget ✓" } else { "OVER 50 ms budget ✗ — reduce WINDOW_LENGTH or HOP_SIZE" });
}
