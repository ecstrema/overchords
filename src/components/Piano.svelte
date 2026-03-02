<script lang="ts">
  import { onMount } from "svelte";
  import { linear, quadInOut } from "svelte/easing";
  import { Tween } from "svelte/motion";

  import { midiToData, type NoteData } from "../lib/midi";
  import type { ActiveNotes } from "../lib/types";

  const centerCMidi = 60;

  export interface Props {
    activeNotes: ActiveNotes;
    onResize?: (width: number, height: number) => void;
    displayStartMidi?: number;
    displayEndMidi?: number;
    midiRangeMode?: "range" | "auto";
  }

  const { activeNotes, onResize = () => {}, displayStartMidi = centerCMidi - 24, displayEndMidi = centerCMidi + 24, midiRangeMode = "range" }: Props = $props();

  // constants for SVG sizing
  const whiteWidth = $state(20);
  const whiteHeight = $state(120);
  const blackWidth = $state(12);
  const blackHeight = $state(80);

  let displayStart = $derived.by(() => (midiRangeMode === "auto" ? Math.min(centerCMidi - 10, ...activeNotes.keys()) - 2 : displayStartMidi));
  let displayEnd = $derived.by(() => (midiRangeMode === "auto" ? Math.max(centerCMidi + 10, ...activeNotes.keys()) + 2 : displayEndMidi));

  function getPosition(midiPosition: number) {
    const data = midiToData(midiPosition);
    if (data.isSharp) {
      return (data.pianoPos + 0.5) * whiteWidth - blackWidth / 2;
    }
    return data.pianoPos * whiteWidth;
  }
  function getEndPosition(midiPosition: number) {
    const data = midiToData(midiPosition);
    if (data.isSharp) {
      return (data.pianoPos + 0.5) * whiteWidth + blackWidth / 2;
    }
    return (data.pianoPos + 1) * whiteWidth;
  }

  const svgViewStart = Tween.of<number>(() => getPosition(displayStart), {
    duration: 100,
    easing: quadInOut,
  });
  const svgViewEnd = Tween.of<number>(() => getEndPosition(displayEnd), {
    duration: (from, to) => (to > from ? 50 : 5000),
    easing: linear,
  });

  $effect(() => {
    onResize(svgViewEnd.current - svgViewStart.current, whiteHeight);
  });

  // determine center C (midi 60) position for dot
  const centerCMidiData = midiToData(centerCMidi);
  const centerCx = $derived.by(
    () => centerCMidiData.pianoPos * whiteWidth + whiteWidth / 2,
  );

  const whiteKeys: (NoteData & { midi: number })[] = [];
  const blackKeys: (NoteData & { midi: number })[] = [];
  for (let i = 0; i < 128; i++) {
    const data = midiToData(i);
    (data.isSharp ? blackKeys : whiteKeys).push({ ...data, midi: i });
  }
</script>

<svg
  class="transition-opacity duration-500 pointer-events-none"
  width={svgViewEnd.current - svgViewStart.current}
  height={whiteHeight}
  viewBox={`${svgViewStart.current} 0 ${svgViewEnd.current - svgViewStart.current} ${whiteHeight}`}
>
  {#each [...whiteKeys, ...blackKeys] as data}
    {@const active = activeNotes.has(data.midi)}
    <rect
      class="transition-colors duration-300"
      fill={active ? "#f00" : data.isSharp ? "#000" : "#fff"}
      stroke="black"
      data-key={data.noteName}
      x={getPosition(data.midi)}
      y="0"
      width={data.isSharp ? blackWidth : whiteWidth}
      height={data.isSharp ? blackHeight : whiteHeight}
    />
  {/each}
  <circle cx={centerCx} cy={whiteHeight - 8} r="4" fill="#000" />
</svg>
