<script setup lang="ts">
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import { useGreeter } from "@/composables/useGreeter";

const { t } = useI18n();
const { sessions, selectedSession } = useGreeter();

const onChange = (event: Event) => {
  const id = (event.target as HTMLSelectElement).value;
  selectedSession.value =
    sessions.value.find((session) => session.id === id) ?? null;
};
</script>

<template>
  <div class="w-full">
    <label
      for="session-select"
      class="text-xs font-semibold text-tx-main uppercase mb-1 block"
    >
      {{ t("login.session") }}
    </label>

    <!-- A single session is not a choice; showing a one-item dropdown is just
         another control to skip past. -->
    <p v-if="sessions.length === 1" class="text-sm text-tx-muted py-2">
      {{ sessions[0].name }}
    </p>

    <select
      v-else-if="sessions.length > 1"
      id="session-select"
      :value="selectedSession?.id"
      @change="onChange"
      class="p-2 border border-ui-border rounded-corner w-full bg-ui-bg/80 text-tx-main focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent"
    >
      <option v-for="session in sessions" :key="session.id" :value="session.id">
        {{ session.name }}
      </option>
    </select>

    <p v-else class="text-sm text-status-warning">
      {{ t("login.noSessions") }}
      <span class="block text-tx-muted">{{ t("login.noSessionsHint") }}</span>
    </p>
  </div>
</template>
