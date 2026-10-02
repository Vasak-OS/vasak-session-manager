<script setup lang="ts">
// TODO(2.4.0): el selector de sesión pasa a la librería (vue-libvasak 2.4.0).
// La 2.3 sólo tiene `SelectField`, que es el `<select>` nativo, y ése es justo
// el que no sirve acá (ver abajo). Hasta entonces la lista es local, con las
// filas de la librería (`ListRow`) y la forma de su campo.
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import { ListRow, ThemeIcon } from "@vasakgroup/vue-libvasak";
import { useGreeter } from "@/composables/useGreeter";
import type { Session } from "@/types/greeter";

const { t } = useI18n();
const { sessions, selectedSession } = useGreeter();

/**
 * A dropdown of our own rather than a `<select>`.
 *
 * The webview draws a native menu for `<select>`, themed by GTK — and the
 * greeter runs before any theme is set up, so the list came out white-on-white
 * over a dark login screen and could not be styled from the page at all.
 */
const open = ref(false);
const highlighted = ref(0);
const root = ref<HTMLElement | null>(null);
const list = ref<HTMLElement | null>(null);

const label = computed(() => selectedSession.value?.name ?? "");

const selectedIndex = computed(() =>
  sessions.value.findIndex(
    (session) => session.id === selectedSession.value?.id,
  ),
);

function choose(session: Session) {
  selectedSession.value = session;
  open.value = false;
}

function toggle() {
  open.value = !open.value;
  if (open.value) highlighted.value = Math.max(selectedIndex.value, 0);
}

/** Keeps the highlighted entry visible in a list long enough to scroll. */
watch(highlighted, async (index) => {
  if (!open.value) return;
  await Promise.resolve();
  list.value?.children[index]?.scrollIntoView({ block: "nearest" });
});

function move(delta: number) {
  if (!open.value) {
    toggle();
    return;
  }
  const count = sessions.value.length;
  highlighted.value = (highlighted.value + delta + count) % count;
}

function onKeydown(event: KeyboardEvent) {
  switch (event.key) {
    case "ArrowDown":
      event.preventDefault();
      move(1);
      break;
    case "ArrowUp":
      event.preventDefault();
      move(-1);
      break;
    case "Enter":
    case " ":
      event.preventDefault();
      if (open.value) {
        const session = sessions.value[highlighted.value];
        if (session) choose(session);
      } else {
        toggle();
      }
      break;
    case "Escape":
      if (open.value) {
        event.preventDefault();
        open.value = false;
      }
      break;
  }
}

/**
 * Los roles van por enlace y no escritos en la plantilla porque el análisis
 * estático (SonarCloud) los juzga sin ver lo que hay detrás: pide `<select>`
 * en vez de `listbox` —y el `<select>` nativo es justo el que sale sin tema en
 * el saludo, ver arriba— y no ve que `ListRow` ya pone `aria-selected` en cada
 * opción (lo comprueba `tests/session-selector.test.ts`).
 */
const LISTBOX = "listbox";
const OPTION = "option";

/**
 * El puntero resalta la opción que tiene debajo. Va delegado en la lista y no
 * en cada fila porque la fila es un componente que no declara ese evento.
 */
function onListPointer(event: MouseEvent) {
  const option = (event.target as HTMLElement | null)?.closest<HTMLElement>(
    '[role="option"]',
  );
  const index = option ? Number(option.id.replace("session-option-", "")) : -1;
  if (index >= 0) highlighted.value = index;
}

/** Clicking anywhere else closes it, the way a real dropdown does. */
function onPointerDown(event: PointerEvent) {
  if (open.value && !root.value?.contains(event.target as Node)) {
    open.value = false;
  }
}

onMounted(() => document.addEventListener("pointerdown", onPointerDown));
onBeforeUnmount(() =>
  document.removeEventListener("pointerdown", onPointerDown),
);
</script>

<template>
  <div ref="root" class="relative w-full min-w-0">
    <!-- La etiqueta chica en mayúsculas de `FormGroup variant="eyebrow"`, a
         mano porque el control es un botón con su valor adentro: un `<label>`
         le pisaría el nombre con el texto de la etiqueta. -->
    <span
      id="session-label"
      class="mb-1 block text-label-xs font-semibold uppercase tracking-wider text-tx-muted"
    >
      {{ t("login.session") }}
    </span>

    <!-- A single session is not a choice; showing a one-item dropdown is just
         another control to skip past. -->
    <p v-if="sessions.length === 1" class="py-2 text-body-s text-tx-muted">
      {{ sessions[0].name }}
    </p>

    <template v-else-if="sessions.length > 1">
      <!-- La forma de `SelectField` de la librería: 32 de alto, el canto de
           3:1 y el anillo de foco del sistema. -->
      <button
        type="button"
        role="combobox"
        aria-controls="session-list"
        aria-haspopup="listbox"
        :aria-expanded="open"
        :aria-activedescendant="open ? `session-option-${highlighted}` : undefined"
        aria-labelledby="session-label"
        @click="toggle"
        @keydown="onKeydown"
        class="flex h-8 w-full min-w-0 items-center justify-between gap-2 rounded-corner-m border border-ui-border-strong bg-ui-surface/70 pr-2 pl-3 text-left text-label-m text-tx-main transition-colors duration-200 ease-ui hover:border-tx-main focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ui-focus"
      >
        <span class="truncate">{{ label }}</span>
        <ThemeIcon name="pan-down-symbolic" type="symbol" :size="16" />
      </button>

      <!-- Lo que flota dentro de la tarjeta va opaco (`ui-float`), como
           cualquier lista desplegable del sistema. -->
      <div
        v-show="open"
        id="session-list"
        ref="list"
        :role="LISTBOX"
        aria-labelledby="session-label"
        class="absolute z-20 mt-1 max-h-56 w-full overflow-y-auto rounded-corner-l border border-ui-line bg-ui-float p-1 shadow-surface-l"
        @mousemove="onListPointer"
      >
        <ListRow
          v-for="(session, index) in sessions"
          :id="`session-option-${index}`"
          :key="session.id"
          :role="OPTION"
          :selected="session.id === selectedSession?.id"
          :title="session.name"
          :class="index === highlighted ? 'bg-ui-hover' : ''"
          @click="choose(session)"
        />
      </div>
    </template>

    <p v-else class="text-body-s text-tx-main">
      {{ t("login.noSessions") }}
      <span class="block text-tx-muted">{{ t("login.noSessionsHint") }}</span>
    </p>
  </div>
</template>
