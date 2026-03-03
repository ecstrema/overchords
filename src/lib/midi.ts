import type { SvelteMap } from "svelte/reactivity";
import type { NoteEvent } from "./types";

export const noteNames = [
  "C",
  "C#",
  "D",
  "D#",
  "E",
  "F",
  "F#",
  "G",
  "G#",
  "A",
  "A#",
  "B",
];

const pianoPosMap = [0, 0.5, 1, 1.5, 2, 3, 3.5, 4, 4.5, 5, 5.5, 6];

export type NoteData = {
  octave: number;
  noteName: string;
  isSharp: boolean;
  pianoPos: number;
  fullName: string;
};

export function midiToData(midi: number): NoteData {
  const octave = Math.floor(midi / 12) - 1;
  const noteIndex = midi % 12;
  const noteName = noteNames[noteIndex];
  const isSharp = noteNames[noteIndex].includes("#");
  const pianoPos = octave * 7 + pianoPosMap[noteIndex];
  const fullName = `${noteName}${octave}`;
  return { octave, noteName, isSharp, pianoPos, fullName };
}

export function filterHarmonics(notes: SvelteMap<number, NoteEvent>): void {
  // iterate over notes, and check if the octave (first harmonic), the 12th (second harmonic), the 15th, the 17th, the 19th, the 21st or the 22nd are preset. Decay by the harmonic number. So the first harmonic should be deduced by half the root's magnitude, the second by a third, the third by a quarter, etc.

  for (const [midi, note] of notes) {
    const rootMagnitude = note.magnitude;
    const harmonics = [1, 12, 15, 17, 19, 21, 22];
    for (let i = 0; i < harmonics.length; i++) {
      const h = harmonics[i];
      const harmonicMidi = midi + h;
      const harmonicNote = notes.get(harmonicMidi);
      if (harmonicNote) {
        const expectedMagnitude = rootMagnitude;
        harmonicNote.magnitude -= expectedMagnitude;
        if (harmonicNote.magnitude < 0) {
          notes.delete(harmonicMidi);
        }
      }
    }
  }
}

export function keepNLoudest(notes: SvelteMap<number, NoteEvent>, n: number): void {
  const sortedNotes = Array.from(notes.entries()).sort((a, b) => b[1].magnitude - a[1].magnitude);
  const valueN = sortedNotes.length > n ? sortedNotes[n - 1][1].magnitude : 0;
  for (const midi of notes.keys()) {
    if (notes.get(midi)!.magnitude < valueN) {
      notes.delete(midi);
    }
  }
}
