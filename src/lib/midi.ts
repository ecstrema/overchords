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
