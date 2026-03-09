# python_tests – Note Detection Benchmark

A self-contained Python benchmark suite that:

1. **Generates** small MIDI files for each test scenario (single notes, intervals, triads, seventh chords, edge cases).
2. **Synthesizes** each MIDI file to WAV using [FluidSynth](https://www.fluidsynth.org/) and a SF2 soundfont.
3. **Runs** every registered note-detection algorithm on the resulting audio.
4. **Reports** per-test precision / recall / F1 and a ranked summary table.

---

## Prerequisites

### FluidSynth

| Platform | Command |
|----------|---------|
| Windows  | `scoop install fluidsynth` or `choco install fluidsynth` |
| Linux    | `sudo apt install fluidsynth` |
| macOS    | `brew install fluid-synth` |

Verify: `fluidsynth --version`

### A Soundfont (.sf2)

Any General-MIDI soundfont works. Free options:

| Soundfont | URL |
|-----------|-----|
| **GeneralUser GS** (recommended, ~30 MB) | https://schristiancollins.com/generaluser.php |
| MuseScore General | https://musescore.org/en/handbook/soundfonts-and-sfz-files |
| FluidR3_GM | ships with `fluid-soundfont-gm` on Debian/Ubuntu |

### Python packages

```bash
uv sync
```

---

## Running the benchmark

```bash
# Minimum: provide the path to your soundfont
uv run run_benchmark.py --soundfont /path/to/GeneralUser.sf2

# Keep the generated MIDI + WAV files for inspection
uv run run_benchmark.py --soundfont ... --keep --output-dir my_output

# Wider tolerance (2 semitones instead of default 1)
uv run run_benchmark.py --soundfont ... --tolerance 2

# Run only triad tests
uv run run_benchmark.py --soundfont ... --filter triad

# List registered algorithms
uv run run_benchmark.py --list-algorithms
```

---

## Project structure

```
python_tests/
├── run_benchmark.py          <- main entry point (CLI)
├── generate_midi.py          <- MIDI file generator (mido)
├── synthesize.py             <- FluidSynth wrapper (MIDI -> WAV)
├── evaluate.py               <- scoring with TP / FP / FN matching
├── report.py                 <- tabulated output and algorithm ranking
├── requirements.txt
│
├── algorithms/
│   ├── base.py               <- DetectorAlgorithm abstract base class
│   ├── cqt_hps.py            <- Rust algorithm port (CQT + HPS)
│   ├── librosa_cqt.py        <- simple CQT detector (no HPS)
│   └── librosa_piptrack.py   <- STFT pitch tracker (librosa.piptrack)
│
└── test_cases/
    └── definitions.py        <- TestCase dataclass + TEST_CASES list
```

---

## Adding a new algorithm

1. Create a new file in `algorithms/`, e.g. `algorithms/my_algo.py`.
2. Subclass `DetectorAlgorithm` from `algorithms/base.py`:

```python
from algorithms.base import DetectorAlgorithm
import numpy as np

class MyAlgorithm(DetectorAlgorithm):
    name = "My algorithm"
    description = "Short description of what it does."

    def detect(self, audio: np.ndarray, sample_rate: int) -> list[int]:
        # ... your implementation ...
        return [60, 64, 67]  # list of MIDI note numbers
```

3. Register it in `run_benchmark.py` inside the `ALGORITHMS` list:

```python
from algorithms.my_algo import MyAlgorithm

ALGORITHMS = [
    ...,
    MyAlgorithm(),
]
```

That's it — it will automatically appear in the next benchmark run and ranking table.

---

## Adding new test cases

Edit `test_cases/definitions.py` and append to `TEST_CASES`:

```python
TestCase("my_chord", [60, 63, 67]),            # C minor triad
TestCase("my_low_note", [24], velocity=90),    # C1
```

---

## Understanding the scoring

For each *(algorithm, test_case)* pair:

| Metric | Definition |
|--------|-----------|
| **TP** (true positives)  | Detected notes that match an expected note within +/-tolerance semitones |
| **FP** (false positives) | Detected notes with no matching expected note |
| **FN** (false negatives) | Expected notes that were not detected |
| **Precision** | TP / (TP + FP) |
| **Recall**    | TP / (TP + FN) |
| **F1**        | 2 * Precision * Recall / (Precision + Recall) |

The **Perfect** column in the ranking shows how many test cases the algorithm
got exactly right (all notes detected, no spurious notes).

The default tolerance is **+/-1 semitone**; use `--tolerance 0` for strict matching.

---

## How the Rust algorithm (CQT+HPS) was ported

The `CqtHpsDetector` in `algorithms/cqt_hps.py` is a faithful translation of
`src-tauri/src/audio.rs`:

| Rust constant | Python equivalent |
|---------------|-------------------|
| `CQT_MIN_FREQ = 8.176` | `_CQT_MIN_FREQ = 8.176` |
| `CQT_MAX_FREQ = 4186.0` | `_CQT_MAX_FREQ = 4186.0` |
| `BINS_PER_OCTAVE = 12` | `_BINS_PER_OCTAVE = 12` |
| `HOP_SIZE = 512` | `_HOP_LENGTH = 512` |
| `MIDI_OFFSET = 0` | `_MIDI_OFFSET = 0` |
| `RELATIVE_THRESHOLD = 0.15` | `_RELATIVE_THRESHOLD = 0.15` |
| `NOTES_TO_KEEP = 5` | `_NOTES_TO_KEEP = 5` |
| `apply_hps_cqt(mags, 2)` | `_apply_hps_cqt(mags, 2)` |

The HPS semitone offsets `[12, 19, 24, 28, 31, 34, 36]` are identical.
The long-term peak normalisation (slow decay) is not replicated because the
benchmark uses single isolated audio clips rather than a live stream.
