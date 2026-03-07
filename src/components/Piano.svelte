<script lang="ts">
  import { linear, quadInOut } from "svelte/easing";
  import { Tween } from "svelte/motion";

  import { midiToData, type NoteData } from "../lib/midi";
  import type { ActiveNotes } from "../lib/types";
  import { getSettingsContext } from "$lib/settings.svelte";

  const centerCMidi = 60;

  export interface Props {
    activeNotes: ActiveNotes;
    onResize?: (width: number, height: number) => void;
    midiRangeMode?: "range" | "auto";
  }

  const {
    activeNotes,
    onResize = () => {},
    midiRangeMode = "range",
  }: Props = $props();

  const settings = getSettingsContext();

  // constants for SVG sizing
  const whiteWidth = $derived.by(() => settings.settings["piano.size"].value);
  const whiteHeight = $derived.by(() => whiteWidth * 6);
  const blackWidth = $derived.by(() => whiteWidth * 0.6);
  const blackHeight = $derived.by(() => (blackWidth * 20) / 3);

  let displayStart = $derived.by(() =>
    midiRangeMode === "auto"
      ? Math.min(centerCMidi - 10, ...activeNotes.keys()) - 2
      : parseInt(settings.settings["piano.range.start"].value, 10),
  );
  let displayEnd = $derived.by(() =>
    midiRangeMode === "auto"
      ? Math.max(centerCMidi + 10, ...activeNotes.keys()) + 2
      : parseInt(settings.settings["piano.range.end"].value, 10),
  );

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
    duration: 50,
    easing: quadInOut,
  });
  const svgViewEnd = Tween.of<number>(() => getEndPosition(displayEnd), {
    duration: (from, to) =>
      midiRangeMode === "range" || to > from ? 50 : 5000,
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

  // Build two 256 length arrays of the fill and stroke colors for each magnitude level (0-255) for white and black keys, to avoid doing calculations in the render loop
  const whiteKeyFillColors = Array.from({ length: 256 }, (_, mag) => {
    const inverseMagHex = (255 - mag).toString(16).padStart(2, "0");
    return `#ff${inverseMagHex}${inverseMagHex}`;
  });
  const blackKeyFillColors = Array.from({ length: 256 }, (_, mag) => {
    return `#${mag.toString(16).padStart(2, "0")}0000`;
  });

  const getFillColor = (midi: number, isSharp: boolean) => {
    const activeNote = activeNotes.get(midi);
    const mag = Math.round(
      255 * (activeNote ? Math.min(activeNote.magnitude, 1) : 0),
    );
    return isSharp ? blackKeyFillColors[mag] : whiteKeyFillColors[mag];
  };
</script>

<svg
  class="transition-opacity duration-500 pointer-events-none"
  width={svgViewEnd.current - svgViewStart.current}
  height={whiteHeight}
  viewBox={`${svgViewStart.current} 0 ${svgViewEnd.current - svgViewStart.current} ${whiteHeight}`}
>
  {#each [...whiteKeys, ...blackKeys] as data}
    <g data-key={data.noteName}>
      <rect
        class="key"
        data-active={activeNotes.has(data.midi)}
        fill={getFillColor(data.midi, data.isSharp)}
        stroke="black"
        x={getPosition(data.midi)}
        y="0"
        width={data.isSharp ? blackWidth : whiteWidth}
        height={data.isSharp ? blackHeight : whiteHeight}
      />
    </g>
  {/each}
  <circle cx={centerCx} cy={whiteHeight - 8} r="4" fill="#000" />
</svg>

<style>
  /* one way transition: fill sets immediately when active, but fades out slowly when inactive */
  /* Not sure it helps... */
  /* .key {
    transition: none;
  }
  .key[data-active="false"] {
    transition: fill 0.1s linear;
  } */
</style>
