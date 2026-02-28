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
  function noteName(midi: number) {
    const note = names[midi % 12];
    const octave = Math.floor(midi / 12) - 1;
    return `${note}${octave}`;
  }
  for (let m = 0; m < 128; m++) {
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
      x = (whiteCount - 1) * whiteWidth + whiteWidth * 0.6;
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
      x={key.x - blackWidth / 2}
      y="0"
      width={blackWidth}
      height={blackHeight}
      fill={activeNotes.includes(key.fullName) ? '#f39' : '#000'}
    />
  {/each}
</svg>

<style>
  .keyboard {
    display: flex;
    position: relative;
    user-select: none;
  }
  .key {
    width: 40px;
    height: 150px;
    border: 1px solid #000;
    box-sizing: border-box;
    background: #fff;
    position: relative;
  }
  .key.sharp {
    width: 30px;
    height: 100px;
    background: #000;
    position: absolute;
    z-index: 1;
  }
  .active {
    background: #f39;
  }
</style>
