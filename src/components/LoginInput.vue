<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import {
  ActionButton,
  AlertMessage,
  FormGroup,
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
const capsLock = ref(false);

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
  // TODO(2.4.0): `TextInput` no declara `spellcheck` ni `autocapitalize`, y con
  // `strictTemplates` no se pueden pasar. Un nombre de cuenta no es una palabra:
  // sin esto el corrector lo subraya y el teclado en pantalla le pone mayúscula.
  const name = document.getElementById("username-field");
  name?.setAttribute("spellcheck", "false");
  name?.setAttribute("autocapitalize", "none");
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

const onPasswordKey = (event: KeyboardEvent) => {
  updateCapsLock(event);
  clearPassword(event);
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
        :placeholder="t('login.usernamePlaceholder')"
      />
    </FormGroup>

    <!-- TODO(2.4.0): el campo de contraseña con «mostrar» pasa a la librería.
         Bloq Mayús va como ayuda del campo: se ve debajo y además se anuncia al
         llegar al campo, que es cuando sirve. -->
    <FormGroup
      :label="t('login.password')"
      variant="eyebrow"
      html-for="password-field"
      :help="capsLock ? t('login.capsLock') : ''"
      v-slot="{ id, describedBy }"
    >
      <TextInput
        :id="id"
        ref="passwordField"
        v-model="password"
        type="password"
        autocomplete="current-password"
        :placeholder="t('login.passwordPlaceholder')"
        :invalid="Boolean(error)"
        :described-by="[describedBy, error ? 'login-error' : ''].filter(Boolean).join(' ') || undefined"
        @keydown="onPasswordKey"
        @keyup="updateCapsLock"
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
