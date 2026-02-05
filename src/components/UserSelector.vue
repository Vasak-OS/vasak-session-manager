<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface User {
  name: string;
  real_name: string;
  uid: number;
  home: string;
  shell: string;
}

defineProps<{ modelValue: User | null }>();
const emit = defineEmits(['update:modelValue']);

const users = ref<User[]>([]);

onMounted(async () => {
  try {
      users.value = await invoke('get_users');
      if (users.value.length > 0) {
          emit('update:modelValue', users.value[0]);
      }
  } catch (e) {
      console.error("Failed to fetch users", e);
  }
});
</script>

<template>
  <div class="flex flex-col gap-2 w-full max-w-sm">
      <h3 class="text-sm font-semibold text-gray-500 mb-2 uppercase">Select User</h3>
      <div v-if="users.length === 0" class="text-gray-400 italic">No users found</div>
      <div v-for="user in users" :key="user.uid" 
           @click="$emit('update:modelValue', user)"
           class="p-4 border rounded-lg cursor-pointer hover:bg-gray-50 transition-colors flex items-center gap-4"
           :class="{'bg-blue-50 border-blue-500 ring-1 ring-blue-500': modelValue?.uid === user.uid, 'border-gray-200': modelValue?.uid !== user.uid}">
          <div class="w-10 h-10 bg-gray-300 rounded-full flex items-center justify-center text-gray-600 font-bold">
              {{ user.name.charAt(0).toUpperCase() }}
          </div>
          <div>
            <div class="font-bold text-gray-800">{{ user.real_name || user.name }}</div>
            <div class="text-xs text-gray-500">@{{ user.name }}</div>
          </div>
      </div>
  </div>
</template>
