<script setup lang="ts">
import { onMounted, onUnmounted, Ref, ref } from "vue";
import { useConfigStore } from "@vasakgroup/plugin-config-manager";
import WindowLayout from "@/layouts/WindowLayout.vue";
import { listen } from "@tauri-apps/api/event";

const configStore = useConfigStore();
let unlistenConfig: Ref<Function | null> = ref(null);

onMounted(async () => {
  configStore.loadConfig();
  unlistenConfig.value = await listen("config-changed", async () => {
    configStore.loadConfig();
  });
});

onUnmounted(() => {
  if (unlistenConfig.value !== null) {
    unlistenConfig.value();
  }
});
</script>

<template>
  <WindowLayout> VAPP </WindowLayout>
</template>
