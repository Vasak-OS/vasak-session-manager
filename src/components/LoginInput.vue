<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import {
  ActionButton,
  AlertMessage,
  FormGroup,
  PasswordField,
  TextInput,
} from "@vasakgroup/vue-libvasak";
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

/** Los campos de la librería exponen `focus()`, que dice si llegó. */
type FocusableField = { focus: () => boolean };
const passwordField = ref<FocusableField | null>(null);
const usernameField = ref<FocusableField | null>(null);

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
 * Escape borra lo escrito en la contraseña, como en cualquier pantalla de
 * inicio: es la forma de empezar de nuevo sin borrar letra por letra algo que
 * no se ve. No envía nada y el foco se queda en el campo.
 */
const clearPassword = (event: KeyboardEvent) => {
  if (event.key !== "Escape") return;
  event.preventDefault();
  password.value = "";
  error.value = "";
};

const login = async () => {
  // Un segundo Enter mientras greetd contesta no manda la contraseña otra vez:
  // greetd tiene una sola conversación abierta, y la segunda llegaría como
  // respuesta a una pregunta que nadie hizo.
  if (loading.value) return;
  if (!username.value) {
    error.value = t("login.usernameRequired");
    await nextTick();
    usernameField.value?.focus();
    return;
  }
  if (!password.value) {
    error.value = t("login.passwordRequired");
    await nextTick();
    passwordField.value?.focus();
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
  <form class="flex w-full min-w-0 flex-col gap-4" @submit.prevent="login">
    <FormGroup
      v-if="usingManualEntry"
      :label="t('login.username')"
      variant="eyebrow"
      html-for="username-field"
      v-slot="{ id }"
    >
      <TextInput
        :id="id"
        ref="usernameField"
        v-model="manualUsername"
        autocomplete="off"
        autocapitalize="none"
        :spellcheck="false"
        :placeholder="t('login.usernamePlaceholder')"
      />
    </FormGroup>

    <!-- El campo de la librería trae el botón de mostrar y el aviso de Bloq
         Mayús, que es la razón más común de que una contraseña correcta se
         rechace; el aviso queda atado al campo (`aria-describedby`). -->
    <FormGroup
      :label="t('login.password')"
      variant="eyebrow"
      html-for="password-field"
      v-slot="{ id, describedBy }"
    >
      <PasswordField
        :id="id"
        ref="passwordField"
        v-model="password"
        autocomplete="current-password"
        :placeholder="t('login.passwordPlaceholder')"
        :caps-lock-label="t('login.capsLock')"
        :invalid="Boolean(error)"
        :described-by="[describedBy, error ? 'login-error' : ''].filter(Boolean).join(' ') || undefined"
        @keydown="clearPassword"
      />
    </FormGroup>

    <p v-if="keyboardHint" class="text-body-xs text-tx-muted">
      {{ keyboardHint }}
      <span v-if="keyboard.switchable"> — {{ t("login.keyboardSwitch") }}</span>
    </p>

    <!-- `role="alert"`, que pone el aviso de error de la librería: se anuncia
         en cuanto aparece, aunque el foco ya haya vuelto al campo. -->
    <div v-if="error" id="login-error">
      <AlertMessage tone="error" icon="auto">
        <span class="break-words">{{ error }}</span>
      </AlertMessage>
    </div>

    <ActionButton
      type="submit"
      variant="primary"
      full-width
      :loading="loading"
      :label="loading ? t('login.authenticating') : t('login.signIn')"
    />
  </form>
</template>
