<script setup lang="ts">
/**
 * La sesión que se va a abrir: el `SearchSelect` de la librería sin buscador.
 *
 * No el `<select>` nativo (`SelectField`): el WebView dibuja ese menú con GTK, y
 * el saludo corre antes de que haya ningún tema, así que la lista salía blanco
 * sobre blanco sobre la pantalla oscura y no se podía tocar desde la página.
 * `SearchSelect` dibuja su propia lista, que flota opaca (`ui-float`) dentro de
 * la tarjeta como cualquier desplegable del sistema.
 */
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import { type OpcionDeBusqueda, SearchSelect } from "@vasakgroup/vue-libvasak";
import { computed } from "vue";
import { useGreeter } from "@/composables/useGreeter";

const { t } = useI18n();
const { sessions, selectedSession } = useGreeter();

const options = computed<OpcionDeBusqueda[]>(() =>
  sessions.value.map((session) => ({ valor: session.id, etiqueta: session.name })),
);

const chosen = computed<string>({
  get: () => selectedSession.value?.id ?? "",
  set: (id) => {
    const session = sessions.value.find((candidate) => candidate.id === id);
    if (session) selectedSession.value = session;
  },
});
</script>

<template>
  <div class="w-full min-w-0">
    <!-- La etiqueta chica en mayúsculas de `FormGroup variant="eyebrow"`. El
         nombre del control lo da `label`, que el botón lleva como
         `aria-label`. -->
    <span
      class="mb-1 block text-label-xs font-semibold uppercase tracking-wider text-tx-muted"
      aria-hidden="true"
    >
      {{ t("login.session") }}
    </span>

    <!-- A single session is not a choice; showing a one-item dropdown is just
         another control to skip past. -->
    <p v-if="sessions.length === 1" class="py-2 text-body-s text-tx-muted">
      {{ sessions[0].name }}
    </p>

    <SearchSelect
      v-else-if="sessions.length > 1"
      v-model="chosen"
      :options="options"
      :label="t('login.session')"
      :searchable="false"
    />

    <p v-else class="text-body-s text-tx-main">
      {{ t("login.noSessions") }}
      <span class="block text-tx-muted">{{ t("login.noSessionsHint") }}</span>
    </p>
  </div>
</template>
