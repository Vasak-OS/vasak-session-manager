<script lang="ts" setup>
import { WindowFrame } from "@vasakgroup/vue-libvasak";
</script>
<template>
    <WindowFrame class="flex rounded-t-window bg-vsk-primary justify-between items-center h-6 h-8 inline-block px-4 py-1">
      <div class="h-[calc(100vh-32px)] rounded-b-window background">
        <slot />
      </div>
    </WindowFrame>
</template>
