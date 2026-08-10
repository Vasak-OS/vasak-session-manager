<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import { useGreeter } from "@/composables/useGreeter";

const { t } = useI18n();
const {
  keyboard,
  selectedSession,
  manualUsername,
  usingManualEntry,
  username,
  rememberChoice,
} = useGreeter();

const password = ref("");
const error = ref("");
const loading = ref(false);
const capsLock = ref(false);

const passwordField = ref<HTMLInputElement | null>(null);
const usernameField = ref<HTMLInputElement | null>(null);

const keyboardHint = computed(() =>
  keyboard.value.layouts.length > 0
    ? t("login.keyboard").replace("{0}", keyboard.value.layouts.join(" · "))
    : "",
);

/** Puts the caret where the person is going to type, without them reaching for
 * the mouse first — the whole screen exists to accept one password. */
const focusEntry = async () => {
  await nextTick();
  if (usingManualEntry.value && !manualUsername.value) {
    usernameField.value?.focus();
  } else {
    passwordField.value?.focus();
  }
};

onMounted(focusEntry);
watch(usingManualEntry, focusEntry);

/**
 * Caps Lock is the single most common reason a correct password is rejected,
 * and a password field gives no other clue. Read on every key event, including
 * the key press that toggles it.
 */
const updateCapsLock = (event: KeyboardEvent) => {
  capsLock.value = event.getModifierState("CapsLock");
};

const login = async () => {
  if (!username.value) {
    error.value = t("login.usernameRequired");
    return;
  }
  if (!password.value) {
    error.value = t("login.passwordRequired");
    return;
  }
  if (!selectedSession.value) {
    error.value = t("login.sessionRequired");
    return;
  }

  loading.value = true;
  error.value = "";

  try {
    rememberChoice();

    // Drive greetd: authenticate and start the session. On success greetd tears
    // this greeter down, so this call does not return.
    await invoke("login", {
      username: username.value,
      password: password.value,
      cmd: selectedSession.value.exec,
      sessionId: selectedSession.value.id,
      sessionType: selectedSession.value.session_type,
      desktopNames: selectedSession.value.desktop_names,
    });
  } catch (e) {
    error.value = String(e);
    password.value = "";
    await nextTick();
    passwordField.value?.focus();
  } finally {
    loading.value = false;
  }
};
</script>

<template>
  <form class="flex flex-col gap-4 w-full" @submit.prevent="login">
    <div v-if="usingManualEntry">
      <label
        for="username-field"
        class="text-xs font-semibold text-tx-main uppercase mb-1 block"
      >
        {{ t("login.username") }}
      </label>
      <input
        id="username-field"
        ref="usernameField"
        v-model="manualUsername"
        type="text"
        autocapitalize="none"
        autocomplete="off"
        spellcheck="false"
        :placeholder="t('login.usernamePlaceholder')"
        class="p-2 border border-ui-border rounded-corner w-full bg-ui-bg/80 text-tx-main focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent"
      />
    </div>

    <div>
      <label
        for="password-field"
        class="text-xs font-semibold text-tx-main uppercase mb-1 block"
      >
        {{ t("login.password") }}
      </label>
      <input
        id="password-field"
        ref="passwordField"
        v-model="password"
        type="password"
        autocomplete="current-password"
        :placeholder="t('login.passwordPlaceholder')"
        @keydown="updateCapsLock"
        @keyup="updateCapsLock"
        class="p-2 border border-ui-border rounded-corner w-full bg-ui-bg/80 text-tx-main focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent"
      />
    </div>

    <p
      v-if="capsLock"
      class="text-status-warning text-sm flex items-center gap-2"
    >
      <span aria-hidden="true">⇧</span>{{ t("login.capsLock") }}
    </p>

    <p v-if="keyboardHint" class="text-tx-muted text-xs">
      {{ keyboardHint }}
      <span v-if="keyboard.switchable"> — {{ t("login.keyboardSwitch") }}</span>
    </p>

    <p
      v-if="error"
      role="alert"
      class="text-status-error text-sm bg-status-error/10 p-2 rounded-corner border border-status-error/30 break-words"
    >
      {{ error }}
    </p>

    <button
      type="submit"
      :disabled="loading"
      class="bg-primary text-tx-on-primary font-semibold py-2 px-4 rounded-corner hover:bg-secondary disabled:opacity-50 disabled:cursor-not-allowed transition-colors shadow-sm"
    >
      {{ loading ? t("login.authenticating") : t("login.signIn") }}
    </button>
  </form>
</template>
