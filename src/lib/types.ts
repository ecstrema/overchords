import type { SvelteMap } from "svelte/reactivity";

export interface NoteEvent {
  midi: number;
  frequency: number;
  magnitude: number;
}

export type ActiveNotes = SvelteMap<number, NoteEvent>
