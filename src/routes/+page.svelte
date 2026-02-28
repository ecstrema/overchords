<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { writable } from 'svelte/store';
  import Piano from '../components/Piano.svelte';

  const activeNotes = writable<string[]>([]);

  onMount(async () => {
    // start backend thread
    await invoke('start_audio_listening');

    const unlisten = await listen<string[]>('notes', (event) => {
      // payload is Vec<NoteEvent>, but serde will convert to array of objects
      // we only care about the `name` field
      const notes = (event.payload as any[]).map((n) => n.name as string);
      activeNotes.set(notes);
    });

    return () => {
      unlisten();
      invoke('stop_audio_listening');
    };
  });
</script>

<main class="container">
  <h1>Overchords</h1>
  <p>Shows notes currently playing on your system in real time.</p>

  <Piano activeNotes={$activeNotes} />
  <p>Currently playing: {$activeNotes.join(', ')}</p>
</main>
