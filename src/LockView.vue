<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import { useConfigStore } from "@vasakgroup/plugin-config-manager";
import type { Store } from "pinia";
import { nextTick, onMounted, ref } from "vue";
import GreeterClock from "@/components/GreeterClock.vue";
import LockIcon from "@/components/LockIcon.vue";
import { useLockScreen } from "@/composables/useLockScreen";

const { t } = useI18n();

const {
  esLaPantallaDelMouse,
  avisos,
  reproduccion,
  punteroAqui,
  ordenarAlReproductor,
  empezar,
} = useLockScreen();

const user = ref("");
const password = ref("");
const error = ref("");
const working = ref(false);
const capsLock = ref(false);
const background = ref<string | null>(null);
const avatar = ref<string | null>(null);
const field = ref<HTMLInputElement | null>(null);

onMounted(async () => {
  // Colours, corner radius and font come from the configuration, the same way
  // every application gets them. This is the screen that has to look most like
  // the rest of the system: it is the one people see without asking for it.
  try {
    user.value = await invoke<string>("lock_user");
    avatar.value = await invoke<string | null>("lock_avatar");
    background.value = await invoke<string | null>("lock_background");
  } catch (reason) {
    // Nunca silencioso: si el puente con Rust no responde, tampoco va a
    // responder el desbloqueo, y hay que verlo antes de quedar encerrado.
    error.value = `lock_user: ${String(reason)}`;
  }

  await empezar();

  await nextTick();
  field.value?.focus();

  // Lo último, y sin bloquear nada de lo anterior: si leer la configuración se
  // cuelga o falla, la pantalla ya está usable con los colores por defecto.
  const configStore = useConfigStore() as Store<
    "config",
    { config: any; loadConfig: () => Promise<void> }
  >;
  configStore.loadConfig().catch(() => {
    // The shipped defaults are still a Vasak screen.
  });
});

/** Caps Lock is the most common reason a correct password is rejected. */
const updateCapsLock = (event: KeyboardEvent) => {
  capsLock.value = event.getModifierState("CapsLock");
};

const submit = async () => {
  if (!password.value || working.value) return;

  working.value = true;
  error.value = "";

  try {
    // A true answer never comes back to a page that still exists: the session
    // is released and the process exits from the Rust side.
    if (!(await invoke<boolean>("unlock", { password: password.value }))) {
      error.value = t("lock.wrongPassword");
      password.value = "";
      await nextTick();
      field.value?.focus();
    }
  } catch {
    error.value = t("lock.error");
  } finally {
    working.value = false;
  }
};
</script>

<template>
  <!-- `mousemove` y `mouseenter`: el compositor manda el segundo cuando la
       superficie aparece debajo del puntero, sin que haga falta mover el mouse,
       y el primero cubre el caso de pasar de un monitor a otro. -->
  <main
    class="relative min-h-screen w-screen flex flex-col items-center justify-center gap-10 bg-ui-surface p-6 select-none overflow-hidden"
    @mouseenter="punteroAqui"
    @mousemove="punteroAqui"
  >
    <!-- El fondo del escritorio, atenuado: se reconoce la sesión que hay
         detrás sin que el texto pierda contraste. -->
    <img
      v-if="background"
      :src="background"
      alt=""
      class="absolute inset-0 h-full w-full object-cover"
    />
    <div v-if="background" class="absolute inset-0 bg-ui-bg/70"></div>

    <!-- La separación tiene que ser mayor que lo que la foto sobresale del
         formulario (-top-12, 48px), o el avatar se le sube encima a la fecha:
         con gap-10 quedaba 40px y se solapaban. -->
    <div class="relative flex flex-col items-center gap-20 w-full">
      <GreeterClock />

    <form
      v-if="esLaPantallaDelMouse"
      class="relative bg-ui-bg/80 px-8 pb-8 pt-14 rounded-corner shadow-xl w-full max-w-md flex flex-col gap-4"
      @submit.prevent="submit"
    >
      <!-- La foto sobresale por encima del borde: es lo que dice de quién es
           esta sesión, sin necesidad de escribir el nombre. -->
      <div
        class="absolute -top-12 left-1/2 -translate-x-1/2 h-24 w-24 rounded-full border-4 border-ui-bg bg-ui-surface shadow-lg overflow-hidden flex items-center justify-center"
      >
        <img v-if="avatar" :src="avatar" alt="" class="h-full w-full object-cover" />
        <span v-else class="text-3xl font-semibold text-tx-muted uppercase">
          {{ user.slice(0, 1) }}
        </span>
      </div>

      <h1 class="text-center text-lg font-semibold text-tx-main">
        {{ t("lock.title") }}
      </h1>

      <div class="flex flex-col gap-1">
        <label
          for="lock-password"
          class="text-xs font-semibold uppercase text-tx-main"
        >
          {{ t("lock.password") }}
        </label>
        <input
          id="lock-password"
          ref="field"
          v-model="password"
          type="password"
          autocomplete="current-password"
          :disabled="working"
          class="p-2 border border-ui-border rounded-corner w-full bg-ui-bg/80 text-tx-main focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent disabled:opacity-50"
          @keydown="updateCapsLock"
          @keyup="updateCapsLock"
        />
      </div>

      <p v-if="capsLock" class="text-status-warning text-sm flex items-center gap-2">
        <span aria-hidden="true">⇧</span>{{ t("lock.capsLock") }}
      </p>

      <p
        v-if="error"
        role="alert"
        class="text-status-error text-sm bg-status-error/10 p-2 rounded-corner border border-status-error/30"
      >
        {{ error }}
      </p>

      <button
        type="submit"
        :disabled="working || !password"
        class="bg-primary text-tx-on-primary font-semibold py-2 px-4 rounded-corner hover:bg-secondary disabled:opacity-50 disabled:cursor-not-allowed transition-colors shadow-sm"
      >
        {{ working ? t("lock.checking") : t("lock.unlock") }}
      </button>
      </form>

      <!-- Qué está esperando la sesión, sin decir qué dice: sólo el icono de
           cada aplicación y cuántos avisos tiene. El contenido no cruza hasta
           una pantalla que puede estar mirando cualquiera. -->
      <div
        v-if="esLaPantallaDelMouse && avisos.length > 0"
        class="flex items-center gap-3"
        :aria-label="t('lock.notifications')"
      >
        <div
          v-for="aviso in avisos"
          :key="aviso.aplicacion"
          class="relative flex h-10 w-10 items-center justify-center rounded-corner bg-ui-bg/70 border border-ui-border"
          :title="t(aviso.cuantas === 1 ? 'lock.notificationsOne' : 'lock.notificationsMany').replace('{0}', String(aviso.cuantas)).replace('{1}', aviso.aplicacion)"
        >
          <LockIcon :name="aviso.icono" :size="22" :alt="aviso.aplicacion" />
          <span
            v-if="aviso.cuantas > 1"
            class="absolute -right-1 -top-1 min-w-5 rounded-full bg-primary px-1 text-center text-[11px] font-medium text-tx-on-primary"
          >
            {{ aviso.cuantas }}
          </span>
        </div>
      </div>

      <!-- El reproductor sólo aparece si algo está sonando: en silencio, esta
           pantalla no tiene por qué decir nada. -->
      <div
        v-if="esLaPantallaDelMouse && reproduccion"
        class="flex items-center gap-3 rounded-corner bg-ui-bg/70 border border-ui-border px-4 py-2 w-full max-w-md"
      >
        <div class="min-w-0 flex-1">
          <p class="truncate text-sm font-medium text-tx-main">{{ reproduccion.titulo }}</p>
          <p v-if="reproduccion.artista" class="truncate text-xs text-tx-muted">
            {{ reproduccion.artista }}
          </p>
        </div>
        <button
          type="button"
          class="flex h-8 w-8 items-center justify-center rounded-corner hover:bg-ui-surface"
          :title="t('lock.pause')"
          :aria-label="t('lock.pause')"
          @click="ordenarAlReproductor('playpause')"
        >
          <LockIcon name="media-playback-pause" :size="18" />
        </button>
        <button
          type="button"
          class="flex h-8 w-8 items-center justify-center rounded-corner hover:bg-ui-surface"
          :title="t('lock.next')"
          :aria-label="t('lock.next')"
          @click="ordenarAlReproductor('next')"
        >
          <LockIcon name="media-skip-forward" :size="18" />
        </button>
      </div>
    </div>
  </main>
</template>
