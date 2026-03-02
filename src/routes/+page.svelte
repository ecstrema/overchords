<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Piano from "../components/Piano.svelte";
  import type { ActiveNotes, NoteEvent } from "../lib/types";
  import { SvelteMap } from "svelte/reactivity";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  let activeNotes: ActiveNotes = new SvelteMap<number, NoteEvent>();

  onMount(async () => {
    // start backend thread
    await invoke("start_audio_listening");

    const unlisten = await listen<string[]>("notes", (event) => {
      const noteEvents = event.payload as any as NoteEvent[];
      activeNotes.clear();
      for (const n of noteEvents) {
        activeNotes.set(n.midi, n);
      }
    });

    return () => {
      unlisten();
      invoke("stop_audio_listening");
    };
  });
</script>

<button
  class="window-button"
  style="top: 4px; right: 4px;"
  onclick={async () => await getCurrentWindow().close()}
>
  X
</button>
<button
  class="window-button"
  style="top: 40px; right: 4px;"
  data-tauri-drag-region
>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width="32"
    height="32"
    viewBox="0 0 24 24"
    ><!-- Icon from Lucide by Lucide Contributors - https://github.com/lucide-icons/lucide/blob/main/LICENSE --><path
      fill="none"
      stroke="currentColor"
      stroke-linecap="round"
      stroke-linejoin="round"
      stroke-width="2"
      d="M3 12h18M3 18h18M3 6h18"
    /></svg
  >
</button>

<main>
  <Piano {activeNotes} />
</main>

<style>
  :global(html, body, main) {
    padding: 0;
    margin: 0;
    background-color: transparent;
  }

  main {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .window-button {
    position: absolute;
    background-color: aliceblue;
    border-radius: 5px;
    border: none;
    cursor: pointer;
    height: 24px;
    width: 24px;
    transition: color 100ms linear;
    user-select: none;
  }

  .window-button:hover {
    background-color: lightblue;
  }
</style>
