<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Piano from "../components/Piano.svelte";
  import type { ActiveNotes, NoteEvent } from "$lib/types";
  import { SvelteMap } from "svelte/reactivity";
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
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

  const resizeToPianoSize = async (width: number, height: number) => {
    await getCurrentWindow().setSize(new LogicalSize(width, height));
  };
</script>

<div class="opacity-30 hover:opacity-100 transition-opacity duration-200">
  <ButtonGroup.Root
    orientation="vertical"
    aria-label="Media controls"
    class="h-fit absolute top-1 right-1"
  >
    <Button
      variant="default"
      size="icon"
      onclick={() => getCurrentWindow().close()}
      title="Close"
    >
      <span class="icon-[lucide--x] h-4 w-4"></span>
    </Button>
    <Button
      variant="default"
      size="icon"
      title="Open Settings"
      onclick={() => {}}
    >
      <span class="icon-[lucide--settings] h-4 w-4"></span>
    </Button>
  </ButtonGroup.Root>

  <main
    data-tauri-drag-region
    class="flex items-center justify-center cursor-move"
  >
    <Piano {activeNotes} onResize={resizeToPianoSize} />
  </main>
</div>
