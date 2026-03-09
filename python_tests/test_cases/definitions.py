"""Test case definitions.

Each TestCase describes a set of simultaneous MIDI notes to play and evaluate.
Add new entries to TEST_CASES to extend the benchmark suite.
"""

from dataclasses import dataclass, field


@dataclass
class TestCase:
    """A single benchmark scenario.

    Attributes:
        name:     Unique identifier used in filenames and reports.
        notes:    MIDI note numbers that sound simultaneously (0–127).
        duration: How long (seconds) each note is held.
        velocity: MIDI velocity for all notes (1–127).
    """
    name: str
    notes: list[int]
    duration: float = 2.0
    velocity: int = 80


# ---------------------------------------------------------------------------
# Benchmark suite
# ---------------------------------------------------------------------------

TEST_CASES: list[TestCase] = [
    # ── Single notes ─────────────────────────────────────────────────────────
    TestCase("single_C2",  [36]),
    TestCase("single_C3",  [48]),
    TestCase("single_C4",  [60]),
    TestCase("single_E4",  [64]),
    TestCase("single_G4",  [67]),
    TestCase("single_A4",  [69]),
    TestCase("single_C5",  [72]),
    TestCase("single_C6",  [84]),

    # ── Two-note intervals ───────────────────────────────────────────────────
    TestCase("interval_maj3rd",     [60, 64]),   # C4 – E4   (major third)
    TestCase("interval_perf5th",    [60, 67]),   # C4 – G4   (perfect fifth)
    TestCase("interval_octave",     [60, 72]),   # C4 – C5   (octave)
    TestCase("interval_min7th",     [60, 70]),   # C4 – Bb4  (minor seventh)
    TestCase("interval_tritone",    [60, 66]),   # C4 – F#4  (tritone – ambiguous)
    TestCase("interval_whole_tone", [60, 62]),   # C4 – D4   (whole tone – hard)
    TestCase("interval_semitone",   [60, 61]),   # C4 – C#4  (semitone – very hard)

    # ── Triads ───────────────────────────────────────────────────────────────
    TestCase("triad_C_major",   [60, 64, 67]),   # C  E  G
    TestCase("triad_A_minor",   [57, 60, 64]),   # A  C  E
    TestCase("triad_G_major",   [55, 59, 62]),   # G  B  D
    TestCase("triad_F_major",   [53, 57, 60]),   # F  A  C
    TestCase("triad_Eb_major",  [51, 55, 58]),   # Eb G  Bb
    TestCase("triad_D_minor",   [50, 53, 57]),   # D  F  A

    # ── Seventh chords ───────────────────────────────────────────────────────
    TestCase("chord_Cmaj7",     [60, 64, 67, 71]),  # C  E  G  B
    TestCase("chord_G7",        [55, 59, 62, 65]),  # G  B  D  F
    TestCase("chord_Dmin7",     [50, 53, 57, 60]),  # D  F  A  C
    TestCase("chord_Am7",       [57, 60, 64, 67]),  # A  C  E  G

    # ── Multi-octave / harmonic-overlap stress tests ─────────────────────────
    TestCase("octaves_c3_c4_c5",  [48, 60, 72]),   # C3 C4 C5 – tests octave errors
    TestCase("power_chord_E",     [40, 47, 52]),    # E2 B2 E3 – guitar power chord
    TestCase("bass_melody",       [36, 64, 67]),    # C2 + E4 G4 – low bass + high chord

    # ── Soft / loud dynamics ─────────────────────────────────────────────────
    TestCase("soft_C4",         [60], velocity=30),
    TestCase("loud_C4",         [60], velocity=120),
    TestCase("soft_triad",      [60, 64, 67], velocity=30),
]
