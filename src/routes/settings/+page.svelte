<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Button } from "$lib/components/ui/button";
  import * as Field from "$lib/components/ui/field";
  import * as Select from "$lib/components/ui/select";
  import Separator from "$lib/components/ui/separator/separator.svelte";

  const close = () => {
    getCurrentWindow().close();
  };

  interface SettingBase {
    name: string;
    value: string;
    defaultValue: string;
    id: string;
    description?: string;
  }

  interface SelectOption {
    value: string;
    label: string;
    description?: string;
  }

  interface SelectSetting extends SettingBase {
    type: "select";
    options: SelectOption[];
  }

  type Setting = SelectSetting;

  const settings: Setting[] = $state([
    {
      id: "theme",
      name: "Theme",
      defaultValue: "system",
      value: localStorage.getItem("theme") || "system",
      type: "select",
      options: [
        { value: "light", label: "Light" },
        { value: "dark", label: "Dark" },
        {
          value: "system",
          label: "System",
          description: "Use the system theme (default)",
        },
      ],
    },
    {
      id: "piano.range.start",
      name: "Piano Range Start",
      defaultValue: "36",
      value: localStorage.getItem("piano.range.start") || "36",
      type: "select",
      options: [
        { value: "0", label: "C0" },
        { value: "24", label: "C1" },
        { value: "36", label: "C2" },
        { value: "60", label: "C3" },
      ],
    },
    {
      id: "piano.range.end",
      name: "Piano Range End",
      defaultValue: "84",
      value: localStorage.getItem("piano.range.end") || "84",
      type: "select",
      options: [
        { value: "72", label: "C4" },
        { value: "84", label: "C5" },
        { value: "96", label: "C6" },
        { value: "108", label: "C7" },
        { value: "120", label: "C8" },
      ],
    },
  ]);

  for (const setting of settings) {
    $effect(() => {
      localStorage.setItem(setting.id, setting.value);
    });
  }

  let hovered = $state<{ name: string; description: string } | undefined>(
    undefined,
  );
  const setHovered = (name: string, description?: string) => {
    hovered = description ? { name, description } : undefined;
  };
  const clearHovered = () => {
    hovered = undefined;
  };
</script>

<div class="w-full max-w-md p-4 mx-auto">
  <h1 class="text-2xl font-bold mb-4">Settings</h1>

  <section>
    <Field.Group>
      {#each Object.entries(settings) as [key, setting] (key)}
        <Field.Field>
          <Field.Content>
            <Field.Label for={`setting-${key}`}>{setting.name}</Field.Label>
            {#if setting.description}
              <Field.Description>{setting.description}</Field.Description>
            {/if}
          </Field.Content>
          <Select.Root type="single" name={key} bind:value={setting.value}>
            <Select.Trigger>
              {setting.options.find((o) => o.value === setting.value)?.label ||
                setting.value}
            </Select.Trigger>
            <Select.Content>
              {#each setting.options as option (option.value)}
                <Select.Item
                  value={option.value}
                  label={option.label}
                  onmouseover={() =>
                    setHovered(option.label, option.description)}
                  onmouseout={() => clearHovered()}
                >
                  {option.label}
                  {option.value === setting.defaultValue ? "(default)" : ""}
                </Select.Item>
              {/each}
              {#if hovered?.description}
                <Separator />
                <div class="p-2 text-sm text-muted-foreground">
                  {hovered.description}
                </div>
              {/if}
            </Select.Content>
          </Select.Root>
        </Field.Field>
      {/each}
    </Field.Group>
  </section>

  <div class="mt-8">
    <Button variant="default" onclick={close}>Close</Button>
  </div>
</div>
