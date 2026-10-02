<script setup lang="ts">
/**
 * El reloj del saludo y de la pantalla de bloqueo: el `ClockDisplay` de la
 * librería sobre una placa translúcida.
 *
 * La placa es lo único propio. Va sobre el fondo de pantalla, que puede ser un
 * video, y desenfoca lo que tiene detrás (`shell-blur`): estas dos pantallas no
 * tienen a Wayfire debajo, así que el desenfoque lo pone la página (decisión
 * del usuario, 02/10/2026).
 *
 * El ancho es fijo y no el del contenido porque `ClockDisplay` es contenedor
 * (`@container`): no aporta ancho propio, y la hora de 60 px (`display-l`)
 * aparece recién cuando el reloj tiene 20rem. 22rem son esos 20 más el margen;
 * en una pantalla más angosta la placa se achica y la hora baja a 48.
 */
import { ClockDisplay } from "@vasakgroup/vue-libvasak";
</script>

<template>
  <div
    data-surface="clock"
    class="w-[22rem] max-w-full rounded-corner-xl border border-ui-line bg-ui-shell px-4 py-3 shadow-surface-s shell-blur"
  >
    <ClockDisplay size="lg" />
  </div>
</template>
