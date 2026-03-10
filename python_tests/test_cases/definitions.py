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
    duration: float = 4.0
    velocity: int = 80


# ---------------------------------------------------------------------------
# Benchmark suite
# ---------------------------------------------------------------------------

TEST_CASES: list[TestCase] = [
    # ── Single notes ─────────────────────────────────────────────────────────
    TestCase("single_C1",  [24]),
    TestCase("single_C2",  [36]),
    TestCase("single_C3",  [48]),
    TestCase("single_C4",  [60]),
    TestCase("single_D4",  [62]),
    TestCase("single_E4",  [64]),
    TestCase("single_F4",  [65]),
    TestCase("single_G4",  [67]),
    TestCase("single_A4",  [69]),
    TestCase("single_B4",  [71]),
    TestCase("single_C5",  [72]),
    TestCase("single_A5",  [81]),
    TestCase("single_C6",  [84]),
    TestCase("single_C7",  [96]),

    # ── Two-note intervals ───────────────────────────────────────────────────
    TestCase("interval_min2nd",     [60, 61]),   # C4 – C#4  (semitone – very hard)
    TestCase("interval_maj2nd",     [60, 62]),   # C4 – D4   (whole tone – hard)
    TestCase("interval_min3rd",     [60, 63]),   # C4 – Eb4  (minor third)
    TestCase("interval_maj3rd",     [60, 64]),   # C4 – E4   (major third)
    TestCase("interval_perf4th",    [60, 65]),   # C4 – F4   (perfect fourth)
    TestCase("interval_tritone",    [60, 66]),   # C4 – F#4  (tritone – ambiguous)
    TestCase("interval_perf5th",    [60, 67]),   # C4 – G4   (perfect fifth)
    TestCase("interval_min6th",     [60, 68]),   # C4 – Ab4  (minor sixth)
    TestCase("interval_maj6th",     [60, 69]),   # C4 – A4   (major sixth)
    TestCase("interval_min7th",     [60, 70]),   # C4 – Bb4  (minor seventh)
    TestCase("interval_maj7th",     [60, 71]),   # C4 – B4   (major seventh)
    TestCase("interval_octave",     [60, 72]),   # C4 – C5   (octave)
    TestCase("interval_2octave",    [48, 72]),   # C3 – C5   (two octaves)

    # ── Triads ───────────────────────────────────────────────────────────────
    TestCase("triad_C_major",        [60, 64, 67]),   # C  E  G
    TestCase("triad_C_minor",        [60, 63, 67]),   # C  Eb G
    TestCase("triad_C_dim",          [60, 63, 66]),   # C  Eb Gb  (diminished)
    TestCase("triad_C_aug",          [60, 64, 68]),   # C  E  G#  (augmented)
    TestCase("triad_C_major_inv1",   [64, 67, 72]),   # E  G  C   (1st inversion)
    TestCase("triad_C_major_inv2",   [67, 72, 76]),   # G  C  E   (2nd inversion)
    TestCase("triad_A_minor",        [57, 60, 64]),   # A  C  E
    TestCase("triad_G_major",        [55, 59, 62]),   # G  B  D
    TestCase("triad_G_minor",        [55, 58, 62]),   # G  Bb D
    TestCase("triad_F_major",        [53, 57, 60]),   # F  A  C
    TestCase("triad_D_major",        [50, 54, 57]),   # D  F# A
    TestCase("triad_D_minor",        [50, 53, 57]),   # D  F  A
    TestCase("triad_Eb_major",       [51, 55, 58]),   # Eb G  Bb
    TestCase("triad_B_major",        [47, 51, 54]),   # B  D# F#

    # ── Seventh chords ───────────────────────────────────────────────────────
    TestCase("chord_Cmaj7",          [60, 64, 67, 71]),  # C  E  G  B  (major 7th)
    TestCase("chord_C7",             [60, 64, 67, 70]),  # C  E  G  Bb (dominant 7th)
    TestCase("chord_Cmin7",          [60, 63, 67, 70]),  # C  Eb G  Bb (minor 7th)
    TestCase("chord_Cdim7",          [60, 63, 66, 69]),  # C  Eb Gb A  (dim 7th – fully symmetric)
    TestCase("chord_Cmin7b5",        [60, 63, 66, 70]),  # C  Eb Gb Bb (half-dim / ø)
    TestCase("chord_G7",             [55, 59, 62, 65]),  # G  B  D  F
    TestCase("chord_Fmaj7",          [53, 57, 60, 64]),  # F  A  C  E
    TestCase("chord_Dmin7",          [50, 53, 57, 60]),  # D  F  A  C
    TestCase("chord_Am7",            [57, 60, 64, 67]),  # A  C  E  G
    TestCase("chord_Cmaj7_low",      [48, 52, 55, 59]),  # C3 E3 G3 B3 (lower register)

    # ── Suspended / added-note chords ────────────────────────────────────────
    TestCase("chord_Csus2",          [60, 62, 67]),       # C  D  G  (sus2)
    TestCase("chord_Csus4",          [60, 65, 67]),       # C  F  G  (sus4)
    TestCase("chord_Cadd9",          [60, 64, 67, 74]),   # C  E  G  D5 (add9)
    TestCase("chord_C6",             [60, 64, 67, 69]),   # C  E  G  A  (6th)

    # ── Multi-octave / harmonic-overlap stress tests ─────────────────────────
    TestCase("octaves_c3_c4_c5",     [48, 60, 72]),   # C3 C4 C5 – octave errors
    TestCase("octaves_c2_c4_c6",     [36, 60, 84]),   # C2 C4 C6 – wide octave spread
    TestCase("power_chord_E",        [40, 47, 52]),   # E2 B2 E3 – guitar power chord
    TestCase("power_chord_A",        [45, 52, 57]),   # A2 E3 A3
    TestCase("bass_melody",          [36, 64, 67]),   # C2 + E4 G4 – low bass + high chord
    TestCase("bass_seventh",         [36, 60, 64, 67, 70]),  # C2 + C4 E4 G4 Bb4

    # ── Dense / cluster chords ───────────────────────────────────────────────
    TestCase("dense_5note",          [60, 62, 64, 67, 69]),  # C D E G A (pentatonic)
    TestCase("dense_6note",          [60, 62, 64, 65, 67, 69]),  # C D E F G A
    TestCase("chromatic_cluster",    [60, 61, 62, 63]),     # C C# D D#  (very hard)
    TestCase("whole_tone_scale",     [60, 62, 64, 66, 68, 70]),  # whole-tone (all ambiguous)

    # ── Soft / loud dynamics ─────────────────────────────────────────────────
    TestCase("soft_C4",              [60], velocity=30),
    TestCase("medium_C4",            [60], velocity=64),
    TestCase("loud_C4",              [60], velocity=120),
    TestCase("soft_triad",           [60, 64, 67], velocity=30),
    TestCase("loud_triad",           [60, 64, 67], velocity=120),
    TestCase("soft_A4",              [69], velocity=30),
    TestCase("soft_seventh",         [60, 64, 67, 71], velocity=30),
]
