import { invoke } from '@tauri-apps/api/core';
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { computed, onUnmounted, ref } from 'vue';

/**
 * Lo que la pantalla de bloqueo sabe de la sesión que hay detrás.
 *
 * Tres cosas, y las tres se piden al lado de Rust:
 *
 * - **Dónde está el mouse.** La pantalla de bloqueo no es una superficie
 *   estirada sobre todos los monitores como el greeter: el protocolo pide una
 *   por salida, así que son páginas separadas que no comparten estado. Se
 *   coordinan por eventos: la que recibe el puntero avisa quién es, y las demás
 *   esconden el formulario.
 * - **Qué aplicaciones tienen avisos sin leer**, sólo el icono y cuántos.
 * - **Si hay algo sonando**, para poder pausarlo sin desbloquear.
 */

/** Una aplicación con notificaciones sin leer. Sin el contenido, a propósito. */
export interface AplicacionConAvisos {
	icono: string;
	aplicacion: string;
	cuantas: number;
}

export interface Reproduccion {
	reproductor: string;
	titulo: string;
	artista: string;
	sonando: boolean;
}

/** Quién tiene el puntero. Lo escuchan todas las pantallas. */
export const EVENTO_PANTALLA_ACTIVA = 'lock:pantalla-activa';

/**
 * Si esta pantalla muestra el formulario.
 *
 * `activa === null` es «nadie tiene el puntero», y ahí se muestran todas: con
 * un compositor que no mande `enter` hasta que el mouse se mueva, la
 * alternativa sería una sesión bloqueada sin ningún lugar donde escribir la
 * contraseña. Es preferible mostrarlo de más que de menos.
 */
export function debeMostrar(propia: string, activa: string | null): boolean {
	return activa === null || activa === propia;
}

/**
 * Si un aviso de hace `desdeHace` milisegundos todavía vale.
 *
 * La pantalla que tiene el puntero lo repite cada tanto. Sin esta caducidad,
 * desconectar el monitor donde estaba el mouse dejaba a las demás escondiendo
 * el formulario para siempre: el aviso de una pantalla que ya no existe no lo
 * contradice nadie.
 */
export function avisoVigente(desdeHace: number): boolean {
	return desdeHace < CADUCIDAD_MS;
}

/** Cada cuánto se vuelve a preguntar por avisos y reproducción. */
const REFRESCO_MS = 5000;

/** Cuánto vale el aviso de quién tiene el puntero: tres refrescos. */
export const CADUCIDAD_MS = REFRESCO_MS * 3;

export function useLockScreen() {
	const etiqueta = getCurrentWindow().label;

	/** La pantalla que tiene el puntero, o `null` mientras nadie lo vio. */
	const pantallaActiva = ref<string | null>(null);
	/** Cuándo llegó el último aviso, para poder dejar de creerle. */
	let ultimoAviso = 0;
	const esLaPantallaDelMouse = computed(() => debeMostrar(etiqueta, pantallaActiva.value));
	const avisos = ref<AplicacionConAvisos[]>([]);
	const reproduccion = ref<Reproduccion | null>(null);

	let dejarDeEscuchar: UnlistenFn | null = null;
	let refresco: ReturnType<typeof setInterval> | null = null;

	/** Esta pantalla tiene el puntero: se lo dice a las demás. */
	function punteroAqui() {
		const yaEraLaActiva = pantallaActiva.value === etiqueta;
		pantallaActiva.value = etiqueta;
		ultimoAviso = Date.now();
		if (yaEraLaActiva) return;
		avisarAlResto();
	}

	function avisarAlResto() {
		emit(EVENTO_PANTALLA_ACTIVA, etiqueta).catch(() => {
			/* Con una sola pantalla no hay a quién avisarle. */
		});
	}

	async function escucharAlResto() {
		dejarDeEscuchar = await listen<string>(EVENTO_PANTALLA_ACTIVA, (evento) => {
			pantallaActiva.value = evento.payload;
			ultimoAviso = Date.now();
		});
	}

	/**
	 * El latido: la pantalla del puntero lo repite, y las demás dejan de creerle
	 * a una que se calló. Es lo que evita quedar sin formulario en ninguna si se
	 * desconecta el monitor donde estaba el mouse.
	 */
	function revisarElAviso() {
		if (pantallaActiva.value === etiqueta) {
			avisarAlResto();
			ultimoAviso = Date.now();
			return;
		}
		if (pantallaActiva.value !== null && !avisoVigente(Date.now() - ultimoAviso)) {
			pantallaActiva.value = null;
		}
	}

	async function refrescarContexto() {
		try {
			avisos.value = await invoke<AplicacionConAvisos[]>('lock_notifications');
		} catch {
			avisos.value = [];
		}
		try {
			reproduccion.value = await invoke<Reproduccion | null>('lock_media');
		} catch {
			reproduccion.value = null;
		}
	}

	async function ordenarAlReproductor(accion: 'playpause' | 'next') {
		const actual = reproduccion.value;
		if (!actual) return;
		try {
			await invoke('lock_media_action', { player: actual.reproductor, action: accion });
		} catch {
			/* El reproductor se fue: el próximo refresco lo saca de la pantalla. */
		}
		// Sin esperar al intervalo: el botón tiene que responder enseguida.
		await refrescarContexto();
	}

	async function empezar() {
		await escucharAlResto();
		await refrescarContexto();
		refresco = setInterval(() => {
			revisarElAviso();
			refrescarContexto();
		}, REFRESCO_MS);
	}

	function terminar() {
		if (dejarDeEscuchar) {
			dejarDeEscuchar();
			dejarDeEscuchar = null;
		}
		if (refresco !== null) {
			clearInterval(refresco);
			refresco = null;
		}
	}

	onUnmounted(terminar);

	return {
		esLaPantallaDelMouse,
		avisos,
		reproduccion,
		punteroAqui,
		ordenarAlReproductor,
		refrescarContexto,
		empezar,
		terminar,
	};
}
