<script setup lang="ts">
/**
 * Suspender, reiniciar y apagar desde la pantalla de inicio: los
 * `PowerActions` de la librería, los mismos que el diálogo de sesión del
 * escritorio, sobre una placa translúcida que desenfoca el fondo de pantalla.
 *
 * Lo propio es sólo qué hace cada uno: los comandos de `commands/power.rs`, que
 * se llaman igual que la acción.
 */
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import { type PowerAction, PowerActions } from "@vasakgroup/vue-libvasak";

const { t } = useI18n();

const actions: PowerAction[] = ["suspend", "reboot", "poweroff"];

const labels = {
  suspend: t("power.suspend"),
  reboot: t("power.reboot"),
  poweroff: t("power.poweroff"),
};

const run = (action: PowerAction) => {
  invoke(action).catch((e) => console.error(`power action '${action}' failed`, e));
};
</script>

<template>
  <div
    data-surface="power"
    class="rounded-corner-l border border-ui-line bg-ui-shell p-1 shadow-surface-xs shell-blur"
  >
    <PowerActions
      :actions="actions"
      :labels="labels"
      button-variant="ghost"
      @action="run"
    />
  </div>
</template>
