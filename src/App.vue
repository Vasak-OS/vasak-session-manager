<script setup lang="ts">
import { ref } from "vue";
import UserSelector from "./components/UserSelector.vue";
import SessionSelector from "./components/SessionSelector.vue";
import LoginInput from "./components/LoginInput.vue";

const selectedUser = ref(null);
const selectedSession = ref(null);
</script>

<template>
  <main class="min-h-screen flex items-center justify-center bg-ui-surface">
    <!-- Center Card -->
    <div class="bg-ui-bg/80 p-8 rounded-corner shadow-xl w-full max-w-4xl flex flex-col md:flex-row gap-8 min-h-[500px]">
        
        <!-- Left: User Selection -->
        <div class="flex-1 flex flex-col border-r border-ui-border pr-8">
            <h1 class="text-2xl font-bold text-text-main mb-6">VasakOS</h1>
            <UserSelector v-model="selectedUser" />
        </div>

        <!-- Right: Login & Options -->
        <div class="flex-1 flex flex-col justify-center gap-6">
            <div v-if="selectedUser" class="text-center md:text-left">
                <h2 class="text-xl font-semibold mb-1">
                    Welcome, <span class="text-blue-600">{{ selectedUser.real_name || selectedUser.name }}</span>
                </h2>
                <p class="text-gray-500 text-sm mb-6">Please enter your credentials to continue.</p>
                
                <SessionSelector v-model="selectedSession" class="mb-4" />
                <LoginInput :user="selectedUser" :session="selectedSession" />
            </div>
            
            <div v-else class="text-center text-gray-400 flex flex-col items-center justify-center h-full">
                <span>Select a user to login</span>
            </div>
        </div>
    </div>
  </main>
</template>

<style>
/* Basic reset/base styles needed if tailwind preflight behaves oddly in webview, but @tailwind base handles it */
</style>
