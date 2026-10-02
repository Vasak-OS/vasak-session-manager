<script setup lang="ts">
// TODO(2.4.0): el reloj grande pasa a la librería (vue-libvasak 2.4.0); hasta
// entonces es local, con tokens. Ojo al adoptarlo: ningún comentario antes de
// la raíz de la plantilla, o se vuelve fragmento y pierde las clases de afuera.
import { onMounted, onUnmounted, ref } from "vue";

const now = ref(new Date());
let alignment: number | undefined;
let timer: number | undefined;

const time = () =>
  now.value.toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  });

const date = () =>
  now.value.toLocaleDateString(undefined, {
    weekday: "long",
    day: "numeric",
    month: "long",
  });

onMounted(() => {
  // Aligned to the next minute, then once a minute: a display that only shows
  // hours and minutes has no reason to wake the machine every second.
  const toNextMinute = 60_000 - (Date.now() % 60_000);
  alignment = window.setTimeout(() => {
    now.value = new Date();
    timer = window.setInterval(() => (now.value = new Date()), 60_000);
  }, toNextMinute);
});

onUnmounted(() => {
  if (alignment !== undefined) window.clearTimeout(alignment);
  if (timer !== undefined) window.clearInterval(timer);
});
</script>

<template>
  <div
    data-surface="clock"
    class="rounded-corner-xl border border-ui-line bg-ui-shell px-4 py-3 text-center shadow-surface-s backdrop-blur-md select-none @xs:px-6"
  >
    <!-- Va sobre el fondo de pantalla, que puede ser un video: la placa
         translúcida con desenfoque es lo que lo mantiene legible cuando pasa un
         cuadro claro. Antes era una sombra de Tailwind sobre el texto, que traía
         su negro fijo. Las dos pantallas no tienen a Wayfire detrás, así que el
         desenfoque lo pone la página (decisión del usuario, 02/10/2026). -->
    <div class="text-5xl font-light text-tx-main tabular-nums @xs:text-6xl">{{ time() }}</div>
    <!-- Only the first letter: `capitalize` would turn "10 de agosto" into
         "10 De Agosto", which is wrong in every language that lowercases its
         month names. -->
    <div class="mt-1 text-body-s text-tx-muted first-letter:uppercase">
      {{ date() }}
    </div>
  </div>
</template>
