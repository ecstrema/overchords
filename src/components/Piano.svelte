<script lang="ts">
  import { derived, get, writable } from "svelte/store";
  export let activeNotes: string[] = [];

  // constants for SVG sizing
  const whiteWidth = 20;
  const whiteHeight = 120;
  const blackWidth = 12;
  const blackHeight = 80;

  // build full 0..127 range (piano has 88 keys, roughly midi 21..108, but we'll show all)
  const keys: {
    midi: number;
    name: string;
    isSharp: boolean;
    fullName: string;
    x: number;
  }[] = [];
  let whiteCount = 0;
  const names = [
    "C",
    "C#",
    "D",
    "D#",
    "E",
    "F",
    "F#",
    "G",
    "G#",
    "A",
    "A#",
    "B",
  ];
  for (let m = 21; m < 108; m++) {
    const name = names[m % 12];
    const isSharp = name.includes("#");
    const octave = Math.floor(m / 12) - 1;
    const fullName = `${name}${octave}`;
    let x: number;
    if (!isSharp) {
      x = whiteCount * whiteWidth;
      whiteCount++;
    } else {
      // place black key slightly to the right of previous white
      x = whiteCount * whiteWidth - blackWidth / 2;
    }
    keys.push({ midi: m, name, isSharp, fullName, x });
  }
  const svgWidth = whiteCount * whiteWidth;
</script>

<svg class="keyboard" width={svgWidth} height={whiteHeight} viewBox={`0 0 ${svgWidth} ${whiteHeight}`}>
  {#each keys.filter(k => !k.isSharp) as key (key.midi)}
    <rect
      x={key.x}
      y="0"
      width={whiteWidth}
      height={whiteHeight}
      fill={activeNotes.includes(key.fullName) ? '#f39' : '#fff'}
      stroke="#000"
    />
  {/each}
  {#each keys.filter(k => k.isSharp) as key (key.midi)}
    <rect
      x={key.x}
      y="0"
      width={blackWidth}
      height={blackHeight}
      fill={activeNotes.includes(key.fullName) ? '#f39' : '#000'}
      stroke="#000"
    />
  {/each}
</svg>
