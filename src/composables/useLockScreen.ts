import { invoke } from '@tauri-apps/api/core';
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { computed, onUnmounted, ref } from 'vue';

/**
 * Lo que la pantalla de bloqueo sabe de la sesión que hay detrás.
 *
 * Tres cosas, y las tres se piden al lado de Rust:
 *
 * - **Cuál de las pantallas es la activa.** La pantalla de bloqueo no es una
 *   superficie estirada sobre todos los monitores como el greeter: el protocolo
 *   pide una por salida, así que son páginas separadas que no comparten estado.
 *   Se coordinan por eventos, y hay tres cosas que pueden reclamar la pantalla:
 *   el foco del teclado —lo avisa Rust, que es quien lo sabe—, el puntero, y una
 *   tecla que llegue a una pantalla que no está mostrando el formulario. Mientras
 *   ninguna reclamó, la de arranque es la del monitor primario.
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

/** Quién reclamó la pantalla. Lo escuchan todas, y también lo emite Rust. */
export const EVENTO_PANTALLA_ACTIVA = 'lock:pantalla-activa';

/**
 * Si esta pantalla muestra el formulario.
 *
 * Tres casos, en orden:
 *
 * 1. **Alguien reclamó el teclado o el puntero** (`activa`): muestra esa y nadie más.
 * 2. **Nadie reclamó nada todavía**, pero Rust dijo cuál es la de arranque
 *    (`porOmision`, el monitor primario): muestra esa. Es el caso normal del
 *    bloqueo por inactividad, donde nadie está tocando el mouse y antes no llegaba
 *    ningún `enter`: el formulario quedaba dibujado en **todas** las pantallas.
 * 3. **Ni una cosa ni la otra** —Rust no pudo resolverlo—: se muestran todas. Es la
 *    salida de emergencia, y sigue estando: una sesión bloqueada sin ningún lugar
 *    donde escribir la contraseña es una máquina que se apaga del botón.
 */
export function debeMostrar(
	propia: string,
	activa: string | null,
	porOmision: string | null = null,
): boolean {
	if (activa !== null) return activa === propia;
	if (porOmision !== null) return porOmision === propia;
	return true;
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

	/** La pantalla que tiene el teclado o el puntero, o `null` mientras nadie lo dijo. */
	const pantallaActiva = ref<string | null>(null);
	/** Cuál dibuja el formulario mientras nadie reclame nada. La resuelve Rust. */
	const pantallaPorOmision = ref<string | null>(null);
	/** Cuándo llegó el último aviso, para poder dejar de creerle. */
	let ultimoAviso = 0;
	const esLaPantallaDelMouse = computed(() =>
		debeMostrar(etiqueta, pantallaActiva.value, pantallaPorOmision.value),
	);
	const avisos = ref<AplicacionConAvisos[]>([]);
	const reproduccion = ref<Reproduccion | null>(null);

	let dejarDeEscuchar: UnlistenFn | null = null;
	let refresco: ReturnType<typeof setInterval> | null = null;

	/**
	 * Esta pantalla tiene el teclado: se lo dice a las demás.
	 *
	 * Es la red de seguridad, y la razón por la que mostrar en una sola pantalla es
	 * seguro. Si el compositor le dio el foco a una superficie que no está
	 * dibujando el formulario, la primera tecla llega igual —al documento— y esa
	 * pantalla reclama. Lo peor que puede pasar es perder esa tecla; sin esto, lo
	 * peor era una sesión en la que no se puede escribir en ninguna parte.
	 */
	function tecladoAqui() {
		punteroAqui();
	}

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

		// Antes de dibujar nada: sin esto la página no sabe en qué pantalla está y
		// muestra el formulario por las dudas, que con varios monitores es
		// mostrarlo en todos.
		try {
			pantallaPorOmision.value = await invoke<string>('lock_active_screen');
		} catch {
			// Se muestran todas, como antes. Es peor que de más, pero nunca de menos.
		}

		window.addEventListener('keydown', tecladoAqui);
		await refrescarContexto();
		refresco = setInterval(() => {
			revisarElAviso();
			refrescarContexto();
		}, REFRESCO_MS);
	}

	function terminar() {
		window.removeEventListener('keydown', tecladoAqui);
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
		pantallaPorOmision,
		tecladoAqui,
		avisos,
		reproduccion,
		punteroAqui,
		ordenarAlReproductor,
		refrescarContexto,
		empezar,
		terminar,
	};
}
