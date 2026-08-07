<script setup lang="ts">
import { ref } from "vue";
import UserSelector from "./components/UserSelector.vue";
import SessionSelector from "./components/SessionSelector.vue";
import LoginInput from "./components/LoginInput.vue";
import PowerMenu from "./components/PowerMenu.vue";

const selectedUser = ref<any>(null);
const selectedSession = ref<any>(null);
</script>

<template>
  <main class="relative min-h-screen flex items-center justify-center bg-ui-surface">
    <!-- Center Card -->
    <div class="bg-ui-bg/80 p-8 rounded-corner shadow-xl w-full max-w-4xl flex flex-col md:flex-row gap-8 min-h-[500px]">

      <!-- Left: User Selection -->
      <div class="flex-1 flex flex-col border-r border-ui-border pr-8">
        <h1 class="text-2xl font-bold text-tx-main mb-6">VasakOS</h1>
        <UserSelector v-model="selectedUser" />
      </div>

      <!-- Right: Login & Options -->
      <div class="flex-1 flex flex-col justify-center gap-6">
        <div v-if="selectedUser" class="text-center md:text-left">
          <h2 class="text-xl font-semibold mb-1 text-tx-main">
            Welcome, <span class="text-primary">{{ selectedUser.real_name || selectedUser.name }}</span>
          </h2>
          <p class="text-tx-muted text-sm mb-6">Please enter your credentials to continue.</p>

          <SessionSelector v-model="selectedSession" class="mb-4" />
          <LoginInput :user="selectedUser" :session="selectedSession" />
        </div>

        <div v-else class="text-center text-tx-muted flex flex-col items-center justify-center h-full">
          <span>Select a user to login</span>
        </div>
      </div>
    </div>

    <!-- Power actions -->
    <PowerMenu class="absolute bottom-6 right-6" />
  </main>
</template>
