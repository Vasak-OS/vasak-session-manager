import { describe, expect, test } from 'bun:test';
import { isClaimFresh, CLAIM_TTL_MS, shouldShowForm } from '../src/composables/useLockScreen';

/**
 * Qué pantalla muestra el formulario cuando hay más de un monitor.
 *
 * La pantalla de bloqueo no es una superficie estirada sobre todas las salidas
 * como el greeter: el protocolo pide una superficie por monitor, así que son
 * páginas separadas que sólo se coordinan por eventos. La decisión de cuál
 * muestra el formulario es esta función, y es la parte que puede dejar a
 * alguien sin ningún lugar donde escribir la contraseña.
 */
describe('qué pantalla muestra el formulario', () => {
	test('sólo la que tiene el puntero', () => {
		expect(shouldShowForm('lock-0', 'lock-0', null)).toBe(true);
		expect(shouldShowForm('lock-1', 'lock-0', null)).toBe(false);
		expect(shouldShowForm('lock-2', 'lock-0', null)).toBe(false);
	});

	test('mientras nadie reclamó nada, la que dijo Rust y sólo esa', () => {
		// El caso normal del bloqueo por inactividad: nadie está tocando el mouse,
		// así que no llega ningún `enter` y sin esto el formulario quedaba dibujado
		// en todas las pantallas a la vez.
		expect(shouldShowForm('lock-0', null, 'lock-0')).toBe(true);
		expect(shouldShowForm('lock-1', null, 'lock-0')).toBe(false);
		expect(shouldShowForm('lock-2', null, 'lock-0')).toBe(false);

		// Y el monitor primario puede no ser el primero.
		expect(shouldShowForm('lock-0', null, 'lock-1')).toBe(false);
		expect(shouldShowForm('lock-1', null, 'lock-1')).toBe(true);
	});

	test('mientras no se sabe cuál es, no la muestra ninguna', () => {
		// El parpadeo: entre que la página monta y que Rust contesta, mostrarla en
		// todas es exactamente el síntoma que esto vino a arreglar. Y la ventana no
		// es corta: la vista espera también a que cargue el fondo de escritorio.
		const screens = ['lock-0', 'lock-1'];
		expect(screens.filter((screen) => shouldShowForm(screen, null, undefined))).toEqual([]);
	});

	test('quien reclama el teclado le gana a la de arranque', () => {
		// Con ext-session-lock el compositor le da el foco a una superficie, y no
		// tiene por qué ser la del monitor primario. Escribir en una pantalla que no
		// muestra el formulario es no poder entrar a la sesión.
		expect(shouldShowForm('lock-1', 'lock-1', 'lock-0')).toBe(true);
		expect(shouldShowForm('lock-0', 'lock-1', 'lock-0')).toBe(false);
		// Y le gana también a «todavía no se sabe»: por eso el oyente de teclado se
		// registra antes de preguntar.
		expect(shouldShowForm('lock-1', 'lock-1', undefined)).toBe(true);
		expect(shouldShowForm('lock-0', 'lock-1', undefined)).toBe(false);
	});

	test('si no se pudo resolver ninguna, se muestran todas', () => {
		// La salida de emergencia, que sigue estando: si la consulta falló o tardó
		// más de SCREEN_WAIT_MS y nadie reclamó, esconder el formulario en todas
		// dejaría la sesión bloqueada sin forma de volver a entrar. `null` es «no se
		// pudo saber», distinto de `undefined`, que es «todavía no se sabe».
		expect(shouldShowForm('lock-0', null, null)).toBe(true);
		expect(shouldShowForm('lock-1', null, null)).toBe(true);
	});

	test('nunca hay más de una mostrando, tampoco en el arranque', () => {
		// La propiedad que importa, y la que estaba rota: para cualquier lista de
		// pantallas y cualquier estado, la cantidad que muestra el formulario es
		// exactamente una mientras se sepa cuál.
		const screens = ['lock-0', 'lock-1', 'lock-2'];
		for (const fallback of screens) {
			const showing = screens.filter((screen) => shouldShowForm(screen, null, fallback));
			expect(showing).toEqual([fallback]);
		}
	});

	test('nunca queda ninguna cuando alguna reclamó el puntero', () => {
		// La propiedad que importa: para cualquier lista de pantallas, siempre
		// hay exactamente una que muestra el formulario.
		const screens = ['lock-0', 'lock-1', 'lock-2'];

		for (const active of screens) {
			const showing = screens.filter((screen) => shouldShowForm(screen, active));
			expect(showing).toEqual([active]);
		}
	});

	test('el monitor que se desconecta no deja a nadie mostrando', () => {
		// Sin caducidad, esto es quedarse afuera de la sesión: la pantalla que
		// tenía el puntero ya no existe, no vuelve a avisar, y las demás siguen
		// escondiendo el formulario para siempre.
		const screens = ['lock-0', 'lock-1'];
		expect(screens.filter((screen) => shouldShowForm(screen, 'lock-9', null))).toEqual([]);

		// Por eso el aviso caduca.
		expect(isClaimFresh(0)).toBe(true);
		expect(isClaimFresh(CLAIM_TTL_MS - 1)).toBe(true);
		expect(isClaimFresh(CLAIM_TTL_MS)).toBe(false);
		expect(isClaimFresh(CLAIM_TTL_MS * 10)).toBe(false);

		// Por eso el aviso caduca: pasado el plazo, la activa vuelve a ser `null` y
		// manda la de arranque, que sigue existiendo.
		const afterExpiry = null;
		expect(screens.filter((screen) => shouldShowForm(screen, afterExpiry, 'lock-0'))).toEqual(['lock-0']);
		// Y sin ninguna de las dos, todas.
		expect(screens.filter((screen) => shouldShowForm(screen, afterExpiry, null))).toEqual(screens);
	});
});
