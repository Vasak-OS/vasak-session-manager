/**
 * El greeter le avisa al splash de arranque que ya pintó; la pantalla de
 * bloqueo no, porque no viene del splash. Se prueba arrancando `main.ts` de
 * verdad con cada URL de ventana.
 */

import { beforeEach, describe, expect, test } from 'bun:test';
import { callsOf, forgetEverything, respond } from './doubles';

/** Arranca `main.ts` como lo haría la ventana con ese hash, y deja correr los frames. */
async function arrancar(hash: string) {
	respond('get_users', []);
	respond('get_sessions', []);
	respond('get_keyboard_layout', { layouts: [], switchable: false });
	respond('get_last_login', { username: null, sessions: {} });
	respond('get_screens', { width: 0, height: 0, screens: [] });
	respond('get_background', { image: null, video: null });
	document.body.innerHTML = '<div id="app"></div>';
	window.location.hash = hash;
	await import(`../src/main.ts?arranque=${hash ? 'bloqueo' : 'greeter'}`);
	// El aviso va en el segundo frame: dos turnos de animación.
	for (let i = 0; i < 3; i++) await new Promise((resolve) => requestAnimationFrame(() => resolve(null)));
}

describe('aviso al splash de arranque', () => {
	beforeEach(() => forgetEverything());

	test('el greeter le avisa al splash que ya pintó', async () => {
		await arrancar('');
		expect(callsOf('dismiss_boot_splash')).toHaveLength(1);
	});

	test('la pantalla de bloqueo no toca el splash', async () => {
		await arrancar('#/lock');
		expect(callsOf('dismiss_boot_splash')).toHaveLength(0);
	});
});
