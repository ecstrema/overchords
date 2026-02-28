<script lang="ts">
  import { onMount } from "svelte";
  import { linear, quadInOut } from 'svelte/easing'
  import { Tween } from "svelte/motion";

  import { midiToData, type NoteData } from "../lib/midi";
  import type { ActiveNotes } from "../lib/types";

  const { activeNotes }: { activeNotes: ActiveNotes } = $props();

  // constants for SVG sizing
  const whiteWidth = $state(20);
  const whiteHeight = $state(120);
  const blackWidth = $state(12);
  const blackHeight = $state(80);

  const centerCMidi = 60;
  let displayStart = $derived.by(() => Math.min(centerCMidi - 10, ...activeNotes.keys()) - 2);
  let displayEnd = $derived.by(() => Math.max(centerCMidi + 10, ...activeNotes.keys()) + 2);

  function clamp(value: number, min: number, max: number) {
    return Math.min(Math.max(value, min), max)
  }

  function setDisplayStart(value: number) {
    displayStart = clamp(value, 0, Math.min(displayEnd - 1, centerCMidi));
  }
  function setDisplayEnd(value: number) {
    displayEnd = clamp(value - 1, Math.max(displayStart + 1, centerCMidi), 128);
  }

  function getPosition(midiPosition: number) {
    const data = midiToData(midiPosition);
    if (data.isSharp) {
      return (data.pianoPos + 0.5) * whiteWidth - blackWidth / 2
    }
    return data.pianoPos * whiteWidth;
  }
  function getEndPosition(midiPosition: number) {
    const data = midiToData(midiPosition);
    if (data.isSharp) {
      return (data.pianoPos + 0.5) * whiteWidth + blackWidth / 2
    }
    return (data.pianoPos + 1) * whiteWidth;
  }

  const svgViewStart = Tween.of<number>(() => getPosition(displayStart), {duration: 100, easing: quadInOut})
  const svgViewEnd = Tween.of<number>(() => getEndPosition(displayEnd), {duration: (from, to) => to > from ? 50: 5000, easing: linear });

  // determine center C (midi 60) position for dot
  const centerCMidiData = midiToData(centerCMidi)
  const centerCx = $derived.by(() => centerCMidiData.pianoPos * whiteWidth + whiteWidth / 2);

  const whiteKeys: (NoteData & {midi: number})[] = []
  const blackKeys: (NoteData & {midi: number})[] = []
  for (let i = 0; i < 128; i++) {
    const data = midiToData(i);
    (data.isSharp ? blackKeys : whiteKeys).push({...data, midi: i});
  }
</script>

<svg
  class="keyboard"
  width={svgViewEnd.current - svgViewStart.current}
  height={whiteHeight}
  viewBox={`${svgViewStart.current} 0 ${svgViewEnd.current - svgViewStart.current} ${whiteHeight}`}
  style:transition={"viewbox 0.5s linear"}
>
  {#each whiteKeys as data}
    {@const active = activeNotes.has(data.midi)}
    <rect
      data-key={data.noteName}
      x={getPosition(data.midi)}
      y="0"
      width={whiteWidth}
      height={whiteHeight}
      fill={active ? "#f39" : "#fff"}
      stroke="#000"
      style={active ? "" : "transition: fill 0.3s linear"}
    />
  {/each}
  {#each blackKeys as data}
    {@const active = activeNotes.has(data.midi)}
    <rect
      data-key={data.noteName}
      x={getPosition(data.midi)}
      y="0"
      width={blackWidth}
      height={blackHeight}
      fill={active ? "#f39" : "#000"}
      stroke="#000"
      style={active ? "" : "transition: fill 0.3s linear"}
    />
  {/each}
  <circle cx={centerCx} cy={whiteHeight - 8} r="4" fill="#000" />
</svg>
