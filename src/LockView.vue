<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import { useConfigStore } from "@vasakgroup/plugin-config-manager";
import {
  ActionButton,
  AlertMessage,
  Avatar,
  Badge,
  FormGroup,
  IconTile,
  PasswordField,
} from "@vasakgroup/vue-libvasak";
import { nextTick, onMounted, ref } from "vue";
import GreeterClock from "@/components/GreeterClock.vue";
import { useLockScreen } from "@/composables/useLockScreen";

const { t } = useI18n();

const { showsForm, notifications, playback, pointerHere, sendToPlayer, start } =
  useLockScreen();

const user = ref("");
const password = ref("");
const error = ref("");
const working = ref(false);
const background = ref<string | null>(null);
const avatar = ref<string | null>(null);
/** El campo de la librería expone `focus()`, que dice si llegó. */
const field = ref<{ focus: () => boolean } | null>(null);

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

  await start();

  await nextTick();
  field.value?.focus();

  // Lo último, y sin bloquear nada de lo anterior: si leer la configuración se
  // cuelga o falla, la pantalla ya está usable con los colores por defecto.
  const configStore = useConfigStore();
  configStore.loadConfig().catch(() => {
    // The shipped defaults are still a Vasak screen.
  });
});

/**
 * Escape borra lo escrito, como en cualquier pantalla de bloqueo: es la forma
 * de empezar de nuevo sin borrar letra por letra algo que no se ve. No envía
 * nada y el foco se queda en el campo.
 */
const onKey = (event: KeyboardEvent) => {
  if (event.key !== "Escape") return;
  event.preventDefault();
  password.value = "";
  error.value = "";
};

/** El texto de cada recuadro de avisos, que es además su nombre accesible. */
const notificationsLabel = (count: number, app: string) =>
  t(count === 1 ? "lock.notificationsOne" : "lock.notificationsMany")
    .replace("{0}", String(count))
    .replace("{1}", app);

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
    }
  } catch {
    error.value = t("lock.error");
  } finally {
    working.value = false;
  }
  // Después de soltar `working`: con el campo apagado el foco no entra, y es
  // ahí donde se va a escribir de nuevo.
  if (error.value) {
    await nextTick();
    field.value?.focus();
  }
};
</script>

<template>
  <main
    class="@container relative h-full w-full overflow-x-hidden overflow-y-auto bg-ui-surface select-none"
    @mouseenter="pointerHere"
    @mousemove="pointerHere"
  >
    <!-- `mousemove` y `mouseenter`: el compositor manda el segundo cuando la
         superficie aparece debajo del puntero, sin que haga falta mover el mouse,
         y el primero cubre el caso de pasar de un monitor a otro.

         Es contenedor y se desplaza por dentro: en una pantalla chica o baja lo
         que no entra se puede alcanzar, en vez de quedar cortado arriba y abajo
         como pasaba con el centrado de antes. -->
    <!-- El fondo del escritorio, atenuado: se reconoce la sesión que hay
         detrás sin que el texto pierda contraste. Fijo, para que no se vaya con
         el desplazamiento. -->
    <img
      v-if="background"
      :src="background"
      alt=""
      class="fixed inset-0 h-full w-full object-cover"
    />
    <div v-if="background" class="fixed inset-0 bg-ui-bg/70"></div>

    <div class="relative flex min-h-full flex-col items-center justify-center p-6">
      <!-- La separación tiene que ser mayor que lo que la foto sobresale del
           formulario (-top-12, 48px), o el avatar se le sube encima a lo que
           tenga arriba —la fecha, o los avisos—: con gap-10 quedaba 40px y se
           solapaban. -->
      <div class="relative flex w-full flex-col items-center gap-20">
        <GreeterClock />

        <!-- Qué está esperando la sesión, sin decir qué dice: sólo el icono de
             cada aplicación y cuántos avisos tiene. El contenido no cruza hasta
             una pantalla que puede estar mirando cualquiera. -->
        <ul
          v-if="showsForm && notifications.length > 0"
          class="flex flex-wrap items-center justify-center gap-3"
          :aria-label="t('lock.notifications')"
        >
          <li
            v-for="entry in notifications"
            :key="entry.app"
            data-surface="notifications"
            class="relative inline-flex rounded-corner-m shell-blur"
            :title="notificationsLabel(entry.count, entry.app)"
          >
            <IconTile
              :name="entry.icon"
              size="md"
              :label="notificationsLabel(entry.count, entry.app)"
            />
            <!-- Oculto al lector: el nombre del recuadro ya dice cuántos. -->
            <span
              v-if="entry.count > 1"
              class="absolute -top-1 -right-1"
              aria-hidden="true"
            >
              <Badge counter variant="solid" tone="accent" :label="entry.count" :max="99" />
            </span>
          </li>
        </ul>

        <form
          v-if="showsForm"
          data-surface="lock-card"
          class="relative flex w-full max-w-md min-w-0 flex-col gap-4 rounded-corner-xl border border-ui-line bg-ui-shell px-4 pt-14 pb-8 shadow-surface-l shell-blur @xs:px-8"
          @submit.prevent="submit"
        >
          <!-- La foto sobresale por encima del borde: es lo que dice de quién
               es esta sesión, sin necesidad de escribir el nombre. El aro es
               del fondo de la ventana, para despegarla de la tarjeta; va como
               contorno para no sumarle tamaño a los 96 de la foto. -->
          <span
            class="absolute -top-12 left-1/2 flex -translate-x-1/2 rounded-corner-full bg-ui-surface shadow-surface-m outline-4 outline-ui-bg"
          >
            <Avatar :src="avatar" :name="user" alt="" size="2xl" />
          </span>

          <h1 class="text-center text-heading-s font-semibold text-tx-main">
            {{ t("lock.title") }}
          </h1>

          <!-- El campo de la librería trae el botón de mostrar y el aviso de
               Bloq Mayús, atado al campo (`aria-describedby`). -->
          <FormGroup
            :label="t('lock.password')"
            variant="eyebrow"
            html-for="lock-password"
            v-slot="{ id, describedBy }"
          >
            <PasswordField
              :id="id"
              ref="field"
              v-model="password"
              autocomplete="current-password"
              :disabled="working"
              :caps-lock-label="t('lock.capsLock')"
              :invalid="Boolean(error)"
              :described-by="[describedBy, error ? 'lock-error' : ''].filter(Boolean).join(' ') || undefined"
              @keydown="onKey"
            />
          </FormGroup>

          <!-- `role="alert"`, que pone el aviso de error de la librería: se
               anuncia en cuanto aparece, aunque el foco ya haya vuelto al campo. -->
          <div v-if="error" id="lock-error">
            <AlertMessage tone="error" icon="auto">
              <span class="break-words">{{ error }}</span>
            </AlertMessage>
          </div>

          <ActionButton
            type="submit"
            variant="primary"
            full-width
            :disabled="!password"
            :loading="working"
            :label="working ? t('lock.checking') : t('lock.unlock')"
          />
        </form>

        <!-- El reproductor sólo aparece si algo está sonando: en silencio, esta
             pantalla no tiene por qué decir nada. -->
        <div
          v-if="showsForm && playback"
          data-surface="player"
          class="flex w-full max-w-md min-w-0 items-center gap-3 rounded-corner-l border border-ui-line bg-ui-shell px-4 py-2 shadow-surface-s shell-blur"
        >
          <div class="min-w-0 flex-1">
            <p class="truncate text-label-m text-tx-main" :title="playback.title">
              {{ playback.title }}
            </p>
            <p
              v-if="playback.artist"
              class="truncate text-body-xs text-tx-muted"
              :title="playback.artist"
            >
              {{ playback.artist }}
            </p>
          </div>
          <ActionButton
            label=""
            variant="ghost"
            icon="media-playback-pause-symbolic"
            :icon-alt="t('lock.pause')"
            :title="t('lock.pause')"
            @click="sendToPlayer('playpause')"
          />
          <ActionButton
            label=""
            variant="ghost"
            icon="media-skip-forward-symbolic"
            :icon-alt="t('lock.next')"
            :title="t('lock.next')"
            @click="sendToPlayer('next')"
          />
        </div>
      </div>
    </div>
  </main>
</template>
