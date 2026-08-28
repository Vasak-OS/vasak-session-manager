<script setup lang="ts">
/**
 * Un icono del tema en la pantalla de bloqueo.
 *
 * `getIconSource` y no `getSymbolSource`: los que se muestran acá son iconos de
 * aplicaciones —el de Telegram, el de Discord— y la variante simbólica los
 * dejaría a todos como una silueta gris, que es justo lo que no sirve cuando lo
 * único que se ve es el icono.
 *
 * El nombre siempre va al plugin; nunca se arma una ruta. Lo que llega es el
 * `app_icon` que mandó la aplicación al notificar, y es dato de afuera.
 */
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { ref, watch } from 'vue';

const props = withDefaults(defineProps<{ name: string; size?: number; alt?: string }>(), {
	size: 20,
	alt: '',
});

const src = ref('');

watch(
	() => props.name,
	async (name) => {
		src.value = name ? await getIconSource(name) : '';
	},
	{ immediate: true },
);
</script>

<template>
  <img
    v-if="src"
    :src="src"
    :alt="alt"
    :style="{ width: `${size}px`, height: `${size}px` }"
    class="object-contain"
  />
</template>
