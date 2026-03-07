import { getContext } from "svelte";

export interface SettingBase<T> {
  name: string;
  defaultValue: T;
  id: string;
  description?: string;
  advanced?: boolean;
}

export interface SelectOption<T> {
  value: T;
  label: string;
  description?: string;
}

export interface SelectSetting extends SettingBase<string> {
  type: "select";
  value: string;
  options: SelectOption<string>[];
}

export interface NumberSetting extends SettingBase<number> {
  type: "number";
  value: number;
  range: [number, number];
  step?: number;
}

export type Setting = SelectSetting | NumberSetting;

export type AllSettings = {
  theme: SelectSetting;
  "piano.range.start": SelectSetting;
  "piano.range.end": SelectSetting;
  "piano.size": NumberSetting;
  "unfocused-opacity": NumberSetting;
  "notes-to-show": NumberSetting;
};

export class Settings {
  settings: AllSettings = $state({
    theme: {
      id: "theme",
      name: "Theme",
      defaultValue: "system",
      advanced: true, // Do not show theme setting in the UI
      value: JSON.parse(localStorage.getItem("theme") || '"system"'),
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
    "piano.range.start": {
      id: "piano.range.start",
      name: "Piano Range Start",
      defaultValue: "36",
      value: JSON.parse(localStorage.getItem("piano.range.start") || '"36"'),
      type: "select",
      options: [
        { value: "0", label: "C-1", description: "Lowest MIDI note" },
        { value: "12", label: "C0" },
        {
          value: "24",
          label: "C1",
          description: "Lowest C on a standard 88-key piano",
        },
        { value: "36", label: "C2" },
        { value: "60", label: "C3" },
      ],
    },
    "piano.range.end": {
      id: "piano.range.end",
      name: "Piano Range End",
      defaultValue: "84",
      value: JSON.parse(localStorage.getItem("piano.range.end") || '"84"'),
      type: "select",
      options: [
        { value: "72", label: "C4" },
        { value: "84", label: "C5" },
        { value: "96", label: "C6" },
        {
          value: "108",
          label: "C7",
          description: "Highest C on a standard 88-key piano",
        },
        { value: "120", label: "C8", description: "Highest MIDI note" },
      ],
    },
    "unfocused-opacity": {
      id: "unfocused-opacity",
      name: "Unfocused Opacity",
      description:
        "Opacity of the main window when it's not focused (default: 50%)",
      defaultValue: 50,
      value: JSON.parse(localStorage.getItem("unfocused-opacity") || "50"),
      type: "number",
      range: [0, 100],
      step: 5,
    },
    "notes-to-show": {
      id: "notes-to-show",
      name: "Notes to Show",
      description:
        "Number of notes to display on the piano (default: 128 - all)",
      defaultValue: 128,
      value: JSON.parse(localStorage.getItem("notes-to-show") || "128"),
      type: "number",
      range: [1, 128],
    },
    "piano.size": {
      id: "piano.size",
      name: "Piano Size",
      description:
        "Size of the piano keys, as determined by the height of a white key (default: 20px)",
      defaultValue: 20,
      value: JSON.parse(localStorage.getItem("piano.size") || "20"),
      type: "number",
      range: [14, 100],
    },
  });

  constructor() {
    addEventListener("storage", (event) => {
      if (event.key === null && event.oldValue === null && event.newValue === null) {
        // This is a clear event, reset all settings to defaults
        this.resetToDefaults();
        return;
      }
      if (event.key && event.key in this.settings) {
        const key = event.key as keyof Settings["settings"];
        const setting = this.settings[key];

        function isEmpty(value: any) {
          return Number.isNaN(value) || value === null || value === undefined;
        }

        if (isEmpty(event.newValue)) {
          setting.value = setting.defaultValue;
          return;
        }

        const parsedValue = JSON.parse(event.newValue!);
        if (isEmpty(parsedValue)) {
          setting.value = setting.defaultValue;
          return;
        }

        if (parsedValue === setting.value) {
          return;
        }

        switch (setting.type) {
          case "select":
            if (setting.options.some((o) => o.value === parsedValue)) {
              setting.value = parsedValue;
            } else {
              setting.value = setting.defaultValue;
            }
            break;
          case "number":
            if (
              typeof parsedValue === "number" &&
              parsedValue >= setting.range[0] &&
              parsedValue <= setting.range[1]
            ) {
              setting.value = parsedValue;
            } else {
              setting.value = setting.defaultValue;
            }
            break;
        }
      }
    });

    for (const key in this.settings) {
      const setting = this.settings[key as keyof typeof this.settings];
      $effect(() => {
        if (setting.value === null || setting.value === undefined) {
          return;
        }
        console.log(`Saving setting ${setting.id} with value ${setting.value}`);
        if (setting.value === setting.defaultValue) {
          localStorage.removeItem(setting.id);
        } else {
          localStorage.setItem(setting.id, JSON.stringify(setting.value));
        }
      });
    }
  }

  resetToDefaults() {
    for (const key in this.settings) {
      const setting = this.settings[key as keyof typeof this.settings];
      setting.value = setting.defaultValue;
    }
  }
}

export function getSettingsContext() {
  const settings = getContext<Settings>("settings");
  if (!settings) {
    throw new Error("Settings not found in context");
  }
  return settings;
}
