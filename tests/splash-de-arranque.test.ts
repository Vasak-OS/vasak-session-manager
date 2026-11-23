/**
 * El greeter le avisa al splash de arranque que ya pintó; la pantalla de
 * bloqueo no, porque no viene del splash. Se prueba arrancando `main.ts` de
 * verdad con cada URL de ventana.
 */

import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { callsOf, forgetEverything, respond } from './doubles';

/** Cola de frames manual: cada `frame()` corre sólo lo que estaba agendado, no lo que se agenda adentro. */
let queue: FrameRequestCallback[] = [];
const realRaf = globalThis.requestAnimationFrame;

function frame() {
	const now = queue;
	queue = [];
	for (const callback of now) callback(0);
}

/** Arranca `main.ts` como lo haría la ventana con ese hash; los frames los avanza la prueba. */
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
}

describe('aviso al splash de arranque', () => {
	beforeEach(() => {
		forgetEverything();
		queue = [];
		const stub = (callback: FrameRequestCallback) => queue.push(callback);
		globalThis.requestAnimationFrame = stub as typeof requestAnimationFrame;
		window.requestAnimationFrame = stub as typeof requestAnimationFrame;
	});
	afterEach(() => {
		globalThis.requestAnimationFrame = realRaf;
		window.requestAnimationFrame = realRaf;
	});

	test('el greeter avisa al splash recién en el segundo frame', async () => {
		await arrancar('');
		expect(callsOf('dismiss_boot_splash')).toHaveLength(0);
		frame();
		expect(callsOf('dismiss_boot_splash')).toHaveLength(0);
		frame();
		expect(callsOf('dismiss_boot_splash')).toHaveLength(1);
	});

	test('la pantalla de bloqueo no toca el splash aunque pasen los frames', async () => {
		await arrancar('#/lock');
		frame();
		frame();
		frame();
		expect(callsOf('dismiss_boot_splash')).toHaveLength(0);
	});
});
