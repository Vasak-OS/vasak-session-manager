<script setup lang="ts">
// TODO(2.4.0): los botones de energía pasan a la librería (vue-libvasak 2.4.0).
// Hasta entonces cada uno es un `ActionButton` de la librería dentro de una
// placa translúcida local.
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import { ActionButton } from "@vasakgroup/vue-libvasak";

const { t } = useI18n();

/**
 * Los iconos son nombres del tema del sistema, no glifos: el `☾ ↻ ⏻` de antes
 * dependía de que la fuente los tuviera, y en el saludo —que corre antes de que
 * haya ninguna configuración— la fuente es la de omisión.
 */
const actions = [
  { cmd: "suspend", label: "power.suspend", icon: "system-suspend-symbolic" },
  { cmd: "reboot", label: "power.reboot", icon: "system-reboot-symbolic" },
  { cmd: "poweroff", label: "power.poweroff", icon: "system-shutdown-symbolic" },
];

const run = (cmd: string) => {
  invoke(cmd).catch((e) => console.error(`power action '${cmd}' failed`, e));
};
</script>

<template>
  <div class="flex gap-2">
    <!-- La placa va por fuera del botón: el botón trae su propio fondo
         (transparente, con el velo al pasar) y dos fondos en el mismo elemento
         no los decide el orden en que se escriben. -->
    <span
      v-for="action in actions"
      :key="action.cmd"
      data-surface="power"
      class="inline-flex rounded-corner-m bg-ui-shell shadow-surface-xs backdrop-blur-md"
    >
      <ActionButton
        label=""
        variant="secondary"
        size="lg"
        :icon="action.icon"
        :icon-alt="t(action.label)"
        :title="t(action.label)"
        @click="run(action.cmd)"
      />
    </span>
  </div>
</template>
