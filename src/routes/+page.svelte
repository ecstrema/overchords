<script lang="ts">
  import { getContext, onMount, unmount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Piano from "../components/Piano.svelte";
  import type { ActiveNotes, NoteEvent } from "$lib/types";
  import { SvelteMap } from "svelte/reactivity";
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { Button } from "$lib/components/ui/button";
  import * as ButtonGroup from "$lib/components/ui/button-group";
  import { getSettingsContext } from "$lib/settings.svelte";
  import { filterHarmonics, keepNLoudest } from "$lib/midi";

  let activeNotes: ActiveNotes = new SvelteMap<number, NoteEvent>();

  let settingsWindow: WebviewWindow | null = null;
  let settingsWindowCloseListener: (() => void) | null = null;

  onMount(() => {
    // start backend thread
    invoke("start_audio_listening");
    let stopNotesListening: (() => void) | null = null;

    listen<string[]>("notes", (event) => {
      const noteEvents = event.payload as any as NoteEvent[];
      activeNotes.clear();
      for (const n of noteEvents) {
        activeNotes.set(n.midi, n);
      }

      filterHarmonics(activeNotes);
      keepNLoudest(activeNotes, settings.settings["notes-to-show"].value);
    }).then((unlisten) => {
      stopNotesListening = unlisten
    });

    // unmount is not called when the window is closed
    const cleanup = () => {
      if (stopNotesListening) stopNotesListening();
      invoke("stop_audio_listening");

      if (settingsWindowCloseListener) settingsWindowCloseListener();
      if (settingsWindow) {
        settingsWindow.close();
      }
    };

    let onWindowCloseUnlistener: (() => void) | null = null;
    getCurrentWindow()
      .onCloseRequested(() => {
        if (onWindowCloseUnlistener) onWindowCloseUnlistener();
        cleanup();
      })
      .then((unlisten) => {
        onWindowCloseUnlistener = unlisten;
      });

    return cleanup;
  });

  const resizeToPianoSize = async (width: number, height: number) => {
    await getCurrentWindow().setSize(new LogicalSize(width, height));
  };

  async function openSettings() {
    if (settingsWindow) {
      settingsWindow.setFocus();
      return;
    }

    // create a new Window and attach a Webview for the settings route
    settingsWindow = new WebviewWindow("settings", {
      url: "settings/",
      title: "Settings",
      width: 400,
      height: 600,
      maximizable: false,
      center: true,
      visible: false, // start hidden to avoid flicker. Plugin window-state will show it once it's ready
    });

    settingsWindowCloseListener = await settingsWindow.onCloseRequested(
      async (e) => {
        settingsWindow = null;
        if (settingsWindowCloseListener) {
          settingsWindowCloseListener();
          settingsWindowCloseListener = null;
        }
      },
    );
  }

  let windowFocused = $state(false);
  getCurrentWindow().onFocusChanged((focused) => {
    windowFocused = focused.payload;
  });

  const settings = getSettingsContext();

  let mounted = $state(false);
  $effect(() => {
    if (mounted) return;
    mounted = true;

    // set initial opacity based on focus state
    getCurrentWindow()
      .isFocused()
      .then((focused) => {
        windowFocused = focused;
      });
  });

  const opacity = $derived.by(() => {
    if (!mounted) return 0;
    return windowFocused
      ? 1
      : settings.settings["unfocused-opacity"].value / 100;
  });

  $effect(() => {
    getCurrentWindow().setIgnoreCursorEvents(
      !windowFocused && settings.settings["unfocused-opacity"].value === 0,
    );
  });
</script>

<div class="hover:opacity-100 transition-opacity duration-200" style:opacity>
  <ButtonGroup.Root
    orientation="vertical"
    aria-label="Media controls"
    class="h-fit absolute top-1 right-1 opacity-60 hover:opacity-100 transition-opacity duration-200"
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
      onclick={openSettings}
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
