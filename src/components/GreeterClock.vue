<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";

const now = ref(new Date());
let alignment: number | undefined;
let timer: number | undefined;

const time = () =>
  now.value.toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  });

const date = () =>
  now.value.toLocaleDateString(undefined, {
    weekday: "long",
    day: "numeric",
    month: "long",
  });

onMounted(() => {
  // Aligned to the next minute, then once a minute: a display that only shows
  // hours and minutes has no reason to wake the machine every second.
  const toNextMinute = 60_000 - (Date.now() % 60_000);
  alignment = window.setTimeout(() => {
    now.value = new Date();
    timer = window.setInterval(() => (now.value = new Date()), 60_000);
  }, toNextMinute);
});

onUnmounted(() => {
  if (alignment !== undefined) window.clearTimeout(alignment);
  if (timer !== undefined) window.clearInterval(timer);
});
</script>

<template>
  <div class="text-center select-none">
    <div class="text-6xl font-light text-tx-main tabular-nums">{{ time() }}</div>
    <!-- Only the first letter: `capitalize` would turn "10 de agosto" into
         "10 De Agosto", which is wrong in every language that lowercases its
         month names. -->
    <div class="text-sm text-tx-muted first-letter:uppercase mt-1">
      {{ date() }}
    </div>
  </div>
</template>
