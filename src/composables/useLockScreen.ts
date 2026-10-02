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
export interface AppNotifications {
	icon: string;
	app: string;
	count: number;
}

export interface Playback {
	player: string;
	title: string;
	artist: string;
	playing: boolean;
}

/** Quién reclamó la pantalla. Lo escuchan todas, y también lo emite Rust. */
export const ACTIVE_SCREEN_EVENT = 'lock:pantalla-activa';

/**
 * Si esta pantalla muestra el formulario.
 *
 * Cuatro casos, en orden:
 *
 * 1. **Alguien reclamó el teclado o el puntero** (`active`): muestra esa y nadie más.
 * 2. **Todavía no se sabe cuál es la de arranque** (`fallback === undefined`): no
 *    la muestra ninguna. Dura lo que tarda Rust en contestar, y es preferible a
 *    mostrarla en todas y que se achique sola: el parpadeo era exactamente el
 *    síntoma que esto vino a arreglar, y la ventana no es corta —la vista espera
 *    también a que cargue el fondo de escritorio, que viaja como data URL—.
 * 3. **Rust dijo cuál es** (`fallback`, el monitor primario): muestra esa. Es el
 *    caso normal del bloqueo por inactividad, donde nadie está tocando el mouse.
 * 4. **No se pudo resolver** (`fallback === null`): se muestran todas. Es la salida
 *    de emergencia, y sigue estando: una sesión bloqueada sin ningún lugar donde
 *    escribir la contraseña es una máquina que se apaga del botón. Se llega ahí si
 *    la consulta falla **o si tarda demasiado**, que es lo que evita que el caso 2
 *    se vuelva permanente.
 *
 * `fallback` no tiene valor por omisión a propósito: con uno, pasarle `undefined`
 * —que es un estado con significado propio— lo reemplazaría por el del parámetro y
 * el caso 2 no se podría ni escribir ni probar.
 */
export function shouldShowForm(
	own: string,
	active: string | null,
	fallback: string | null | undefined,
): boolean {
	if (active !== null) return active === own;
	if (fallback === undefined) return false;
	if (fallback !== null) return fallback === own;
	return true;
}

/**
 * Si un aviso de hace `age` milisegundos todavía vale.
 *
 * La pantalla que tiene el puntero lo repite cada tanto. Sin esta caducidad,
 * desconectar el monitor donde estaba el mouse dejaba a las demás escondiendo
 * el formulario para siempre: el aviso de una pantalla que ya no existe no lo
 * contradice nadie.
 */
export function isClaimFresh(age: number): boolean {
	return age < CLAIM_TTL_MS;
}

/** Cada cuánto se vuelve a preguntar por avisos y reproducción. */
const REFRESH_MS = 5000;

/** Cuánto vale el aviso de quién tiene el puntero: tres refrescos. */
export const CLAIM_TTL_MS = REFRESH_MS * 3;

/**
 * Cuánto se espera a que Rust diga cuál es la pantalla de arranque.
 *
 * Pasado el plazo se muestran todas. No es por prolijidad: si el comando no
 * contestara nunca, sin esto no quedaría **ninguna** pantalla donde escribir.
 */
export const SCREEN_WAIT_MS = 3000;

export function useLockScreen() {
	const label = getCurrentWindow().label;

	/** La pantalla que tiene el teclado o el puntero, o `null` mientras nadie lo dijo. */
	const activeScreen = ref<string | null>(null);
	/**
	 * Cuál dibuja el formulario mientras nadie reclame nada. La resuelve Rust.
	 *
	 * `undefined` es «todavía no se sabe» y `null` es «no se pudo saber»: la
	 * diferencia importa, porque el primero no muestra en ninguna y el segundo
	 * muestra en todas.
	 */
	const fallbackScreen = ref<string | null | undefined>(undefined);
	/** Cuándo llegó el último aviso, para poder dejar de creerle. */
	let lastClaim = 0;
	const showsForm = computed(() =>
		shouldShowForm(label, activeScreen.value, fallbackScreen.value),
	);
	const notifications = ref<AppNotifications[]>([]);
	const playback = ref<Playback | null>(null);

	let stopListening: UnlistenFn | null = null;
	let refreshTimer: ReturnType<typeof setInterval> | null = null;

	/**
	 * Esta pantalla tiene el teclado: se lo dice a las demás.
	 *
	 * Es la red de seguridad, y la razón por la que mostrar en una sola pantalla es
	 * seguro. Si el compositor le dio el foco a una superficie que no está
	 * dibujando el formulario, la primera tecla llega igual —al documento— y esa
	 * pantalla reclama. Lo peor que puede pasar es perder esa tecla; sin esto, lo
	 * peor era una sesión en la que no se puede escribir en ninguna parte.
	 */
	function keyboardHere() {
		pointerHere();
	}

	/** Esta pantalla tiene el puntero: se lo dice a las demás. */
	function pointerHere() {
		const wasActive = activeScreen.value === label;
		activeScreen.value = label;
		lastClaim = Date.now();
		if (wasActive) return;
		tellOthers();
	}

	function tellOthers() {
		emit(ACTIVE_SCREEN_EVENT, label).catch(() => {
			/* Con una sola pantalla no hay a quién avisarle. */
		});
	}

	async function listenToOthers() {
		stopListening = await listen<string>(ACTIVE_SCREEN_EVENT, (event) => {
			activeScreen.value = event.payload;
			lastClaim = Date.now();
		});
	}

	/**
	 * El latido: la pantalla del puntero lo repite, y las demás dejan de creerle
	 * a una que se calló. Es lo que evita quedar sin formulario en ninguna si se
	 * desconecta el monitor donde estaba el mouse.
	 */
	function checkClaim() {
		if (activeScreen.value === label) {
			tellOthers();
			lastClaim = Date.now();
			return;
		}
		if (activeScreen.value !== null && !isClaimFresh(Date.now() - lastClaim)) {
			activeScreen.value = null;
		}
	}

	async function refreshContext() {
		try {
			// `?? []`: una respuesta vacía no puede tirar abajo el dibujo de la
			// pantalla, y con él el formulario para desbloquear.
			notifications.value = (await invoke<AppNotifications[] | null>('lock_notifications')) ?? [];
		} catch {
			notifications.value = [];
		}
		try {
			playback.value = (await invoke<Playback | null>('lock_media')) ?? null;
		} catch {
			playback.value = null;
		}
	}

	async function sendToPlayer(action: 'playpause' | 'next') {
		const current = playback.value;
		if (!current) return;
		try {
			await invoke('lock_media_action', { player: current.player, action: action });
		} catch {
			/* El reproductor se fue: el próximo refresco lo saca de la pantalla. */
		}
		// Sin esperar al intervalo: el botón tiene que responder enseguida.
		await refreshContext();
	}

	/**
	 * Pregunta en qué pantalla se dibuja, con su plazo.
	 *
	 * Arranca en cuanto se usa el composable y no dentro de `start()`, que la
	 * vista llama recién después de pedir el usuario, el avatar y el fondo: hacerla
	 * esperar a eso dejaba el formulario dibujado en todas las pantallas mientras
	 * tanto.
	 */
	function askForScreen(): Promise<void> {
		const deadline = setTimeout(() => {
			if (fallbackScreen.value === undefined) fallbackScreen.value = null;
		}, SCREEN_WAIT_MS);

		return invoke<string>('lock_active_screen')
			.then((screen) => {
				fallbackScreen.value = screen;
			})
			.catch(() => {
				// Se muestran todas. Es peor que de más, pero nunca de menos.
				fallbackScreen.value = null;
			})
			.finally(() => clearTimeout(deadline));
	}

	// Las dos cosas que no pueden esperar a `start()`: saber dónde dibujar, y
	// poder reclamar la pantalla con una tecla si se dibujó en la equivocada.
	window.addEventListener('keydown', keyboardHere);
	const screenAnswer = askForScreen();

	async function start() {
		await listenToOthers();
		await screenAnswer;
		await refreshContext();
		refreshTimer = setInterval(() => {
			checkClaim();
			void refreshContext();
		}, REFRESH_MS);
	}

	function stop() {
		window.removeEventListener('keydown', keyboardHere);
		if (stopListening) {
			stopListening();
			stopListening = null;
		}
		if (refreshTimer !== null) {
			clearInterval(refreshTimer);
			refreshTimer = null;
		}
	}

	onUnmounted(stop);

	return {
		showsForm,
		fallbackScreen,
		keyboardHere,
		notifications,
		playback,
		pointerHere,
		sendToPlayer,
		refreshContext,
		start,
		stop,
	};
}
