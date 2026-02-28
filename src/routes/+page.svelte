<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Piano from "../components/Piano.svelte";
  import type { ActiveNotes, NoteEvent } from "../lib/types";
  import { midiToData } from "../lib/midi";
  import { SvelteMap } from "svelte/reactivity";

  let activeNotes: ActiveNotes = new SvelteMap<number, NoteEvent>();

  onMount(async () => {
    // start backend thread
    await invoke("start_audio_listening");

    const unlisten = await listen<string[]>("notes", (event) => {
      const noteEvents = event.payload as any as NoteEvent[];
      activeNotes.clear();
      for (const n of noteEvents) {
        activeNotes.set(n.midi, n)
      }
    });

    return () => {
      unlisten();
      invoke("stop_audio_listening");
    };
  });
</script>

<main style="display: flex; align-items: center; justify-content: center;" data-tauri-drag-region>
  <Piano activeNotes={activeNotes} />
</main>

<style>
  :global(html, body, main) {
    padding: 0;
    margin: 0;
    background-color: transparent;
  }
</style>
