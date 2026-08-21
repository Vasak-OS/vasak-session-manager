<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import GreeterClock from "@/components/GreeterClock.vue";
import LoginInput from "@/components/LoginInput.vue";
import PowerMenu from "@/components/PowerMenu.vue";
import SessionSelector from "@/components/SessionSelector.vue";
import UserSelector from "@/components/UserSelector.vue";
import { displayName, useGreeter } from "@/composables/useGreeter";

const { t } = useI18n();
const {
  load,
  activeScreen,
  background,
  backgroundVideoUrl,
  releaseBackgroundVideo,
  layout,
  pointerMoved,
  selectedUser,
  usingManualEntry,
  users,
} = useGreeter();

/**
 * The greeter is one surface stretched across every monitor, so each monitor is
 * a rectangle inside the page. With no monitors reported — running it in a
 * window during development, or a compositor that tells us nothing — the whole
 * surface is treated as a single screen.
 */
const panels = computed(() =>
  layout.value.screens.length > 0
    ? layout.value.screens.map((screen) => ({
        key: screen.index,
        style: {
          left: `${screen.x}px`,
          top: `${screen.y}px`,
          width: `${screen.width}px`,
          height: `${screen.height}px`,
        },
      }))
    : [{ key: 0, style: { inset: "0" } }],
);

/**
 * Where the login box sits: the monitor holding the pointer.
 *
 * It is one element that moves, not one per monitor, so a password half typed
 * survives a nudge of the mouse — remounting it on the other screen would take
 * the focus and the typing with it.
 */
const loginArea = computed(
  () =>
    panels.value.find((panel) => panel.key === activeScreen.value)?.style ??
    panels.value[0].style,
);

const wallpaper = computed(() =>
  background.value ? { backgroundImage: `url("${background.value}")` } : {},
);

/**
 * El video no se pudo reproducir después de todo —contenedor que el
 * decodificador no acepta, archivo cortado—: se suelta, y el fondo vuelve a ser
 * la imagen que ya está debajo, en vez de quedar una pantalla negra.
 */
const onVideoError = () => {
  console.error(
    "El fondo en movimiento no se pudo reproducir; se muestra la imagen.",
  );
  releaseBackgroundVideo();
};

onMounted(load);
</script>

<template>
  <main
    class="fixed inset-0 overflow-hidden bg-ui-bg"
    @mousemove="pointerMoved($event.clientX, $event.clientY)"
  >
    <!-- One wallpaper per monitor rather than one stretched across all of
         them, which on a two-screen desk shows half a photograph on each. -->
    <div
      v-for="panel in panels"
      :key="panel.key"
      class="absolute bg-ui-bg bg-cover bg-center overflow-hidden"
      :style="{ ...panel.style, ...wallpaper }"
    >
      <!-- El fondo en movimiento va encima de la imagen, no en su lugar: la
           imagen es lo que se ve mientras el video llega y lo que queda si
           falla. Uno por monitor, como la imagen, aunque eso sea un decodificador
           por pantalla: es el precio de no mostrar medio cuadro en cada una, y
           esta pantalla vive unos segundos. -->
      <video
        v-if="backgroundVideoUrl"
        :src="backgroundVideoUrl"
        class="absolute inset-0 h-full w-full object-cover"
        autoplay
        loop
        muted
        playsinline
        disablepictureinpicture
        tabindex="-1"
        aria-hidden="true"
        @error="onVideoError"
      ></video>

      <!-- La misma atenuación que el resto del sistema usa sobre un fondo, con
           el color de la interfaz y no uno fijo: es lo que le da contraste al
           texto sobre cualquier foto o video. -->
      <div class="absolute inset-0 bg-ui-bg/40"></div>
    </div>

    <div
      class="absolute flex flex-col items-center justify-center gap-10 p-6 transition-all duration-300 ease-out"
      :style="loginArea"
    >
      <GreeterClock />

      <!-- Las mismas transparencias que cualquier superficie de VasakOS:
           `bg-ui-bg/80` con `backdrop-blur-md` y el borde de la interfaz. El
           blur es lo que deja que se vea el fondo sin que compita con el
           formulario. -->
      <div
        class="bg-ui-bg/80 backdrop-blur-md border border-ui-border p-8 rounded-corner shadow-xl w-full max-w-4xl flex flex-col md:flex-row gap-8"
      >
        <!-- Accounts. Hidden when there is nobody to choose between, so a
             single-user machine goes straight to the password. -->
        <div
          v-if="users.length > 0"
          class="flex-1 flex flex-col md:border-r md:border-ui-border md:pr-8 max-h-[60vh] overflow-y-auto"
        >
          <h1 class="text-2xl font-bold text-tx-main mb-6">
            {{ t("login.title") }}
          </h1>
          <UserSelector />
        </div>

        <div class="flex-1 flex flex-col justify-center gap-6">
          <h1 v-if="users.length === 0" class="text-2xl font-bold text-tx-main">
            {{ t("login.title") }}
          </h1>

          <div v-if="selectedUser" class="flex items-center gap-4">
            <img
              v-if="selectedUser.avatar"
              :src="selectedUser.avatar"
              alt=""
              class="w-14 h-14 rounded-full object-cover"
            />
            <h2 class="text-xl font-semibold text-tx-main">
              {{ displayName(selectedUser) }}
            </h2>
          </div>

          <p
            v-if="usingManualEntry && users.length === 0"
            class="text-tx-muted text-sm"
          >
            {{ t("login.noUsersHint") }}
          </p>

          <SessionSelector />
          <LoginInput />
        </div>
      </div>

      <PowerMenu class="absolute bottom-6 right-6" />
    </div>
  </main>
</template>
