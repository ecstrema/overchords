<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Button } from "$lib/components/ui/button";
  import * as Field from "$lib/components/ui/field";
  import * as Select from "$lib/components/ui/select";
  import Separator from "$lib/components/ui/separator/separator.svelte";
  import { Input } from "$lib/components/ui/input";
  import { getSettingsContext } from "$lib/settings.svelte";

  const settings = getSettingsContext();

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

<svelte:head>
  <title>Overchords - Settings</title>
  <meta name="description" content="Configure your Overchords settings" />
</svelte:head>

<div class="w-full max-w-md p-4 mx-auto">
  <h1 class="text-2xl font-bold mb-4">Settings</h1>

  <section>
    <Field.Group>
      {#each Object.values(settings.settings) as setting (setting.id)}
        {#if !setting.advanced}
          <Field.Field>
            <Field.Content>
              <Field.Label for={`setting-${setting.id}`}>
                {setting.name}
              </Field.Label>
              {#if setting.description}
                <Field.Description>{setting.description}</Field.Description>
              {/if}
            </Field.Content>
            {#if setting.type === "select"}
              <Select.Root
                type="single"
                name={setting.id}
                bind:value={setting.value}
              >
                <Select.Trigger>
                  {setting.options.find((o) => o.value === setting.value)
                    ?.label || setting.value}
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
            {:else if setting.type === "number"}
              <Input
                id={`setting-${setting.id}`}
                type="number"
                min={setting.range[0]}
                max={setting.range[1]}
                step={setting.step || 1}
                bind:value={setting.value}
                class="w-full"
              />
            {/if}
          </Field.Field>
        {/if}
      {/each}
    </Field.Group>
  </section>

  <div class="mt-8 flex justify-between">
    <Button variant="default" onclick={async () => await getCurrentWindow().close()}>
      Close
    </Button>
    <Button
      variant="outline"
      onclick={() => settings.resetToDefaults()}
    >
      Reset to defaults
    </Button>
  </div>
</div>
