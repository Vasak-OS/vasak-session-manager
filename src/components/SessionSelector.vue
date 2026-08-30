<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
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
  <div ref="root" class="w-full relative">
    <span
      id="session-label"
      class="text-xs font-semibold text-tx-main uppercase mb-1 block"
    >
      {{ t("login.session") }}
    </span>

    <!-- A single session is not a choice; showing a one-item dropdown is just
         another control to skip past. -->
    <p v-if="sessions.length === 1" class="text-sm text-tx-muted py-2">
      {{ sessions[0].name }}
    </p>

    <template v-else-if="sessions.length > 1">
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
        class="p-2 border border-ui-border rounded-corner w-full bg-ui-bg/80 backdrop-blur-md text-tx-main text-left flex items-center justify-between gap-2 focus:ring-2 focus:ring-primary focus:border-transparent"
      >
        <span class="truncate">{{ label }}</span>
        <span class="text-tx-muted text-xs shrink-0" aria-hidden="true">▾</span>
      </button>

      <ul
        v-show="open"
        id="session-list"
        ref="list"
        role="listbox"
        aria-labelledby="session-label"
        class="absolute z-20 mt-1 w-full max-h-56 overflow-y-auto rounded-corner border border-ui-border bg-ui-surface/90 backdrop-blur-md shadow-xl py-1"
      >
        <li
          v-for="(session, index) in sessions"
          :id="`session-option-${index}`"
          :key="session.id"
          role="option"
          :aria-selected="session.id === selectedSession?.id"
          @click="choose(session)"
          @mousemove="highlighted = index"
          class="px-3 py-2 cursor-pointer text-sm text-tx-main"
          :class="[
            index === highlighted ? 'bg-secondary/30' : '',
            session.id === selectedSession?.id ? 'font-semibold' : '',
          ]"
        >
          {{ session.name }}
        </li>
      </ul>
    </template>

    <p v-else class="text-sm text-status-warning">
      {{ t("login.noSessions") }}
      <span class="block text-tx-muted">{{ t("login.noSessionsHint") }}</span>
    </p>
  </div>
</template>
