<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";

const { t } = useI18n();

const actions = [
  { cmd: "suspend", label: "power.suspend", glyph: "☾" },
  { cmd: "reboot", label: "power.reboot", glyph: "↻" },
  { cmd: "poweroff", label: "power.poweroff", glyph: "⏻" },
];

const run = (cmd: string) => {
  invoke(cmd).catch((e) => console.error(`power action '${cmd}' failed`, e));
};
</script>

<template>
  <div class="flex gap-2">
    <button
      v-for="action in actions"
      :key="action.cmd"
      type="button"
      :title="t(action.label)"
      :aria-label="t(action.label)"
      @click="run(action.cmd)"
      class="w-10 h-10 rounded-corner bg-ui-bg/80 border border-ui-border text-tx-main text-lg leading-none hover:bg-primary hover:text-tx-on-primary transition-colors"
    >
      {{ action.glyph }}
    </button>
  </div>
</template>
