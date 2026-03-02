<script lang="ts">
  import { ModeWatcher, resetMode, setMode } from "mode-watcher";
  import "./layout.css";

  const { children } = $props();

  // listen for settings updates from the settings window
  addEventListener("storage", (event) => {
    if (event.key === "theme") {
      if (event.newValue === "light" || event.newValue === "dark") {
        setMode(event.newValue as "light" | "dark");
      } else {
        resetMode();
      }
    }
  });
</script>

<ModeWatcher />
{@render children()}
