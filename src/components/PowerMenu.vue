<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";

const actions = [
  { cmd: "suspend", label: "Suspend", glyph: "☾" },
  { cmd: "reboot", label: "Restart", glyph: "↻" },
  { cmd: "poweroff", label: "Shut down", glyph: "⏻" },
];

const run = (cmd: string) => {
  invoke(cmd).catch((e) => console.error(`power action '${cmd}' failed`, e));
};
</script>

<template>
  <div class="flex gap-2">
    <button
      v-for="a in actions"
      :key="a.cmd"
      :title="a.label"
      :aria-label="a.label"
      @click="run(a.cmd)"
      class="w-10 h-10 rounded-corner bg-ui-bg/80 border border-ui-border text-tx-main text-lg leading-none hover:bg-primary hover:text-tx-on-primary transition-colors"
    >
      {{ a.glyph }}
    </button>
  </div>
</template>
