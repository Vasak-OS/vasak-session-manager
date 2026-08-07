<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface Session {
  id: string;
  name: string;
  exec: string;
  session_type: string;
}

defineProps<{ modelValue: Session | null }>();
const emit = defineEmits(['update:modelValue']);

const sessions = ref<Session[]>([]);

onMounted(async () => {
    try {
        sessions.value = await invoke('get_sessions');
        if (sessions.value.length > 0) {
            emit('update:modelValue', sessions.value[0]);
        }
    } catch(e) {
        console.error("Failed to load sessions", e);
    }
});
</script>
<template>
    <div class="w-full max-w-sm">
        <label class="text-xs font-semibold text-tx-main uppercase mb-1 block">Session</label>
        <select :value="modelValue?.id" 
                @change="e => {
                    const s = sessions.find(s => s.id === (e.target as HTMLSelectElement).value);
                    if(s) $emit('update:modelValue', s);
                }"
                class="p-2 border border-ui-border rounded-corner w-full bg-ui-bg/80 text-tx-main focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent">
            <option v-for="s in sessions" :key="s.id" :value="s.id">
                {{ s.name }} ({{ s.session_type }})
            </option>
        </select>
    </div>
</template>
