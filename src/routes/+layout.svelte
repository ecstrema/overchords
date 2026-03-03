<script lang="ts">
  import "./layout.css";
  import { Settings } from "$lib/settings.svelte";
  import { setContext } from "svelte";
  import { MediaQuery } from "svelte/reactivity";

  const { children } = $props();

  const settings = new Settings();
  setContext("settings", settings);

  const themeClass = $derived.by(() => {
    const themeSetting = settings.settings.theme.value;
    if (themeSetting === "light" || themeSetting === "dark") {
      return themeSetting;
    } else {
      // if theme is set to "auto", use the system preference
      const prefersDark = new MediaQuery("(prefers-color-scheme: dark)");
      return prefersDark.current ? "dark" : "light";
    }
  });

  $effect(() => {
    document.documentElement.classList.remove("light", "dark");
    document.documentElement.classList.add(themeClass);

    document.documentElement.style.setProperty(
      "color-scheme",
      themeClass === "dark" ? "dark" : "light",
    );
  });

  const computedBackground = $derived.by(() => {
    settings.settings.theme.value; // re-run when theme changes
    return getComputedStyle(document.documentElement).getPropertyValue("--background");
  });
</script>

<svelte:head>
  <meta
    name="theme-color"
    content={computedBackground}
  />
</svelte:head>

{@render children()}
