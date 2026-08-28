import { describe, expect, test } from 'bun:test';
import { avisoVigente, CADUCIDAD_MS, debeMostrar } from '../src/composables/useLockScreen';

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
		expect(debeMostrar('lock-0', 'lock-0')).toBe(true);
		expect(debeMostrar('lock-1', 'lock-0')).toBe(false);
		expect(debeMostrar('lock-2', 'lock-0')).toBe(false);
	});

	test('mientras nadie vio el puntero, se muestran todas', () => {
		// Es la salida de emergencia: con un compositor que no mande `enter`
		// hasta que el mouse se mueva, esconder el formulario en todas dejaría la
		// sesión bloqueada sin forma de volver a entrar. Es preferible mostrarlo
		// de más que de menos.
		expect(debeMostrar('lock-0', null)).toBe(true);
		expect(debeMostrar('lock-1', null)).toBe(true);
	});

	test('nunca queda ninguna cuando alguna reclamó el puntero', () => {
		// La propiedad que importa: para cualquier lista de pantallas, siempre
		// hay exactamente una que muestra el formulario.
		const pantallas = ['lock-0', 'lock-1', 'lock-2'];

		for (const activa of pantallas) {
			const mostrando = pantallas.filter((pantalla) => debeMostrar(pantalla, activa));
			expect(mostrando).toEqual([activa]);
		}
	});

	test('el monitor que se desconecta no deja a nadie mostrando', () => {
		// Sin caducidad, esto es quedarse afuera de la sesión: la pantalla que
		// tenía el puntero ya no existe, no vuelve a avisar, y las demás siguen
		// escondiendo el formulario para siempre.
		const pantallas = ['lock-0', 'lock-1'];
		expect(pantallas.filter((pantalla) => debeMostrar(pantalla, 'lock-9'))).toEqual([]);

		// Por eso el aviso caduca: pasado el plazo, la activa vuelve a ser
		// `null` y todas muestran de nuevo.
		expect(avisoVigente(0)).toBe(true);
		expect(avisoVigente(CADUCIDAD_MS - 1)).toBe(true);
		expect(avisoVigente(CADUCIDAD_MS)).toBe(false);
		expect(avisoVigente(CADUCIDAD_MS * 10)).toBe(false);

		const trasCaducar = null;
		expect(pantallas.filter((pantalla) => debeMostrar(pantalla, trasCaducar))).toEqual(pantallas);
	});
});
