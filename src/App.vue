<script setup lang="ts">
import { onMounted } from "vue";
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import GreeterClock from "@/components/GreeterClock.vue";
import LoginInput from "@/components/LoginInput.vue";
import PowerMenu from "@/components/PowerMenu.vue";
import SessionSelector from "@/components/SessionSelector.vue";
import UserSelector from "@/components/UserSelector.vue";
import { displayName, useGreeter } from "@/composables/useGreeter";

const { t } = useI18n();
const { load, selectedUser, usingManualEntry, users } = useGreeter();

onMounted(load);
</script>

<template>
  <main
    class="relative min-h-screen flex flex-col items-center justify-center gap-10 bg-ui-surface p-6"
  >
    <GreeterClock />

    <div
      class="bg-ui-bg/80 p-8 rounded-corner shadow-xl w-full max-w-4xl flex flex-col md:flex-row gap-8"
    >
      <!-- Accounts. Hidden when there is nobody to choose between, so a
           single-user machine goes straight to the password. -->
      <div
        v-if="users.length > 0"
        class="flex-1 flex flex-col md:border-r md:border-ui-border md:pr-8 max-h-[60vh] overflow-y-auto"
      >
        <h1 class="text-2xl font-bold text-tx-main mb-6">
          {{ t("login.title") }}
        </h1>
        <UserSelector />
      </div>

      <div class="flex-1 flex flex-col justify-center gap-6">
        <h1 v-if="users.length === 0" class="text-2xl font-bold text-tx-main">
          {{ t("login.title") }}
        </h1>

        <div v-if="selectedUser" class="flex items-center gap-4">
          <img
            v-if="selectedUser.avatar"
            :src="selectedUser.avatar"
            alt=""
            class="w-14 h-14 rounded-full object-cover"
          />
          <h2 class="text-xl font-semibold text-tx-main">
            {{ displayName(selectedUser) }}
          </h2>
        </div>

        <p v-if="usingManualEntry && users.length === 0" class="text-tx-muted text-sm">
          {{ t("login.noUsersHint") }}
        </p>

        <SessionSelector />
        <LoginInput />
      </div>
    </div>

    <PowerMenu class="absolute bottom-6 right-6" />
  </main>
</template>
