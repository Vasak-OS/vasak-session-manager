<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const props = defineProps<{ user: any, session: any }>();
const password = ref('');
const error = ref('');
const loading = ref(false);

const login = async () => {
    if (!props.user || !props.session) {
        error.value = "Please select a user and session.";
        return;
    }
    loading.value = true;
    error.value = '';
    
    try {
        // Drive greetd: authenticate + start the session. On success greetd
        // tears this greeter down and starts the session, so this call may not
        // return; any rejection is an auth/session error to surface.
        await invoke('login', {
            username: props.user.name,
            password: password.value,
            cmd: props.session.exec,
            sessionType: props.session.session_type,
        });
    } catch (e) {
        error.value = String(e);
        password.value = ''; // Clear password on failure
    } finally {
        loading.value = false;
    }
};
</script>
<template>
    <div class="flex flex-col gap-4 w-full max-w-sm">
        <div>
            <label class="text-xs font-semibold text-tx-main uppercase mb-1 block">Password</label>
            <input type="password" v-model="password" 
                @keyup.enter="login"
                class="p-2 border border-ui-border rounded-corner w-full focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent" 
                placeholder="Enter password..." />
        </div>
        
        <div v-if="error" class="text-status-error text-sm bg-status-error/10 p-2 rounded-corner border border-status-error/30">
            {{ error }}
        </div>

        <button @click="login" :disabled="loading || !user"
                class="bg-primary text-tx-on-primary font-semibold py-2 px-4 rounded-corner hover:bg-secondary disabled:opacity-50 disabled:cursor-not-allowed transition-colors shadow-sm">
            {{ loading ? 'Authenticating...' : 'Login' }}
        </button>
    </div>
</template>
