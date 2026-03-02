<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Piano from "../components/Piano.svelte";
  import type { ActiveNotes, NoteEvent } from "$lib/types";
  import { SvelteMap } from "svelte/reactivity";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Button } from "$lib/components/ui/button";
  import * as ButtonGroup from "$lib/components/ui/button-group";

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

<div class="opacity-30 hover:opacity-100 transition-opacity duration-100">
  <ButtonGroup.Root
    orientation="vertical"
    aria-label="Media controls"
    class="h-fit absolute top-0 right-4"
  >
    <Button
      variant="outline"
      size="icon"
      aria-label="Close"
      onclick={() => getCurrentWindow().close()}
    >
      <span class="icon-[lucide--x] h-4 w-4"></span>
    </Button>
    <Button
      variant="outline"
      size="icon"
      aria-label="Settings"
      onclick={() => {}}
    >
      <span class="icon-[lucide--settings] h-4 w-4"></span>
    </Button>
  </ButtonGroup.Root>

  <main
    class="flex items-center justify-center cursor-move"
    data-tauri-drag-region
  >
    <Piano {activeNotes} />
  </main>
</div>
