<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import { Avatar } from "@vasakgroup/vue-libvasak";
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
        disablePictureInPicture
        tabindex="-1"
        aria-hidden="true"
        @error="onVideoError"
      ></video>

      <!-- La misma atenuación que el resto del sistema usa sobre un fondo, con
           el color de la interfaz y no uno fijo: es lo que le da contraste al
           texto sobre cualquier foto o video. -->
      <div class="absolute inset-0 bg-ui-bg/40"></div>
    </div>

    <!-- La pantalla donde va el cuadro. Es contenedor (`@container`) porque lo
         que decide si las columnas van lado a lado es el ancho de **este**
         monitor, no el de la superficie entera, y WebKitGTK no avisa de
         `matchMedia` ni de `resize`. Y se desplaza por dentro: en una pantalla
         chica el cuadro apilado no entra, y sin esto quedaban cortados el reloj
         arriba y el botón de entrar abajo. -->
    <div
      class="@container absolute overflow-x-hidden overflow-y-auto transition-all duration-300 ease-ui-out"
      :style="loginArea"
    >
      <div class="flex min-h-full flex-col items-center justify-center gap-10 p-2 @xs:p-6">
        <GreeterClock />

        <!-- La tarjeta va translúcida y desenfoca el fondo que tiene detrás:
             esta pantalla no tiene a Wayfire debajo que lo haga, así que lo hace
             la página (decisión del usuario, 02/10/2026). A partir de 720 px de
             pantalla (el `md:` de antes, menos el margen) las cuentas y la
             contraseña van lado a lado; más angosta, una columna debajo de la
             otra. -->
        <div
          data-surface="card"
          class="flex w-full max-w-4xl min-w-0 flex-col gap-8 rounded-corner-xl border border-ui-line bg-ui-shell p-2 shadow-surface-l shell-blur @xs:p-8 @min-[45rem]:flex-row"
        >
          <!-- Accounts. Hidden when there is nobody to choose between, so a
               single-user machine goes straight to the password. -->
          <div
            v-if="users.length > 0"
            class="flex min-w-0 flex-1 flex-col @min-[45rem]:max-h-[60vh] @min-[45rem]:overflow-y-auto @min-[45rem]:border-r @min-[45rem]:border-ui-line @min-[45rem]:pr-8"
          >
            <h1 class="mb-6 text-2xl font-bold break-words text-tx-main">
              {{ t("login.title") }}
            </h1>
            <UserSelector />
          </div>

          <div class="flex min-w-0 flex-1 flex-col justify-center gap-6">
            <h1 v-if="users.length === 0" class="text-2xl font-bold break-words text-tx-main">
              {{ t("login.title") }}
            </h1>

            <!-- La foto, si la cuenta tiene una: sin foto no se dibuja nada,
                 como antes. `!size-14`: 56 como antes (la 2.3 tiene 48 y 64). -->
            <div v-if="selectedUser" class="flex min-w-0 items-center gap-4">
              <Avatar
                v-if="selectedUser.avatar"
                :src="selectedUser.avatar"
                :name="displayName(selectedUser)"
                alt=""
                size="xl"
                class="!size-14"
              />
              <h2 class="min-w-0 text-xl font-semibold break-words text-tx-main">
                {{ displayName(selectedUser) }}
              </h2>
            </div>

            <p
              v-if="usingManualEntry && users.length === 0"
              class="text-body-s text-tx-muted"
            >
              {{ t("login.noUsersHint") }}
            </p>

            <SessionSelector />
            <LoginInput />
          </div>
        </div>

        <!-- En la esquina cuando hay lugar; en una pantalla angosta, debajo de
             la tarjeta, que es donde no la pisa. -->
        <PowerMenu
          class="self-end @min-[45rem]:absolute @min-[45rem]:right-6 @min-[45rem]:bottom-6"
        />
      </div>
    </div>
  </main>
</template>
