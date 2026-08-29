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
 * Cuatro casos, en orden:
 *
 * 1. **Alguien reclamó el teclado o el puntero** (`activa`): muestra esa y nadie más.
 * 2. **Todavía no se sabe cuál es la de arranque** (`porOmision === undefined`): no
 *    la muestra ninguna. Dura lo que tarda Rust en contestar, y es preferible a
 *    mostrarla en todas y que se achique sola: el parpadeo era exactamente el
 *    síntoma que esto vino a arreglar, y la ventana no es corta —la vista espera
 *    también a que cargue el fondo de escritorio, que viaja como data URL—.
 * 3. **Rust dijo cuál es** (`porOmision`, el monitor primario): muestra esa. Es el
 *    caso normal del bloqueo por inactividad, donde nadie está tocando el mouse.
 * 4. **No se pudo resolver** (`porOmision === null`): se muestran todas. Es la salida
 *    de emergencia, y sigue estando: una sesión bloqueada sin ningún lugar donde
 *    escribir la contraseña es una máquina que se apaga del botón. Se llega ahí si
 *    la consulta falla **o si tarda demasiado**, que es lo que evita que el caso 2
 *    se vuelva permanente.
 *
 * `porOmision` no tiene valor por omisión a propósito: con uno, pasarle `undefined`
 * —que es un estado con significado propio— lo reemplazaría por el del parámetro y
 * el caso 2 no se podría ni escribir ni probar.
 */
export function debeMostrar(
	propia: string,
	activa: string | null,
	porOmision: string | null | undefined,
): boolean {
	if (activa !== null) return activa === propia;
	if (porOmision === undefined) return false;
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

/**
 * Cuánto se espera a que Rust diga cuál es la pantalla de arranque.
 *
 * Pasado el plazo se muestran todas. No es por prolijidad: si el comando no
 * contestara nunca, sin esto no quedaría **ninguna** pantalla donde escribir.
 */
export const ESPERA_PANTALLA_MS = 3000;

export function useLockScreen() {
	const etiqueta = getCurrentWindow().label;

	/** La pantalla que tiene el teclado o el puntero, o `null` mientras nadie lo dijo. */
	const pantallaActiva = ref<string | null>(null);
	/**
	 * Cuál dibuja el formulario mientras nadie reclame nada. La resuelve Rust.
	 *
	 * `undefined` es «todavía no se sabe» y `null` es «no se pudo saber»: la
	 * diferencia importa, porque el primero no muestra en ninguna y el segundo
	 * muestra en todas.
	 */
	const pantallaPorOmision = ref<string | null | undefined>(undefined);
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

	/**
	 * Pregunta en qué pantalla se dibuja, con su plazo.
	 *
	 * Arranca en cuanto se usa el composable y no dentro de `empezar()`, que la
	 * vista llama recién después de pedir el usuario, el avatar y el fondo: hacerla
	 * esperar a eso dejaba el formulario dibujado en todas las pantallas mientras
	 * tanto.
	 */
	function preguntarLaPantalla(): Promise<void> {
		const plazo = setTimeout(() => {
			if (pantallaPorOmision.value === undefined) pantallaPorOmision.value = null;
		}, ESPERA_PANTALLA_MS);

		return invoke<string>('lock_active_screen')
			.then((cual) => {
				pantallaPorOmision.value = cual;
			})
			.catch(() => {
				// Se muestran todas. Es peor que de más, pero nunca de menos.
				pantallaPorOmision.value = null;
			})
			.finally(() => clearTimeout(plazo));
	}

	// Las dos cosas que no pueden esperar a `empezar()`: saber dónde dibujar, y
	// poder reclamar la pantalla con una tecla si se dibujó en la equivocada.
	window.addEventListener('keydown', tecladoAqui);
	const laPantalla = preguntarLaPantalla();

	async function empezar() {
		await escucharAlResto();
		await laPantalla;
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
