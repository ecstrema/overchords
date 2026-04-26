import type { SvelteMap } from "svelte/reactivity";

export interface NoteEvent {
  midi: number;
  probability: number;
}

export type ActiveNotes = SvelteMap<number, NoteEvent>
