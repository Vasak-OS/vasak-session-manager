/**
 * La sección `lock.*` de `vasak.conf` y lo que cambia en la pantalla de bloqueo.
 *
 * Dos cosas tienen que valer a la vez: sin `lock` la pantalla se dibuja como
 * siempre, y ninguna combinación de claves deja la sesión sin el campo de
 * contraseña ni el avatar. Se mira lo que se dibuja, no las funciones internas.
 */

import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { createPinia, setActivePinia } from 'pinia';
import LockView from '../src/LockView.vue';
import { useLockConfig } from '../src/composables/useLockConfig';
import { forgetEverything, respond } from './doubles';

let wrapper: VueWrapper | null = null;

const PICTURE = 'data:image/png;base64,AAAA';
const PLAYER = { player: 'org.mpris.MediaPlayer2.x', title: 'Tema', artist: 'Alguien', playing: true };
const NOTIFICATIONS = [{ icon: 'telegram', app: 'Telegram', count: 3 }];

/** Monta la pantalla con la sesión «con de todo»: fondo, reproductor y avisos. */
async function open(lock?: unknown, extra: Record<string, unknown> = {}) {
	respond('lock_user', 'pato');
	respond('lock_avatar', 'data:image/png;base64,BBBB');
	respond('lock_background', PICTURE);
	respond('lock_media', PLAYER);
	respond('lock_notifications', NOTIFICATIONS);
	respond('lock_active_screen', 'lock-0');
	// `loadConfig()` lee `vasak.conf` por este comando del plugin.
	respond('plugin:config-manager|read_config', lock === undefined ? null : JSON.stringify({ ...extra, lock }));
	wrapper = mount(LockView, { attachTo: document.body, global: { plugins: [createPinia()] } });
	await flushPromises();
	return wrapper;
}

const clock = () => document.querySelector('[data-clock]') as HTMLElement | null;
const time = () => clock()?.querySelector('time') as HTMLElement;
const date = () => clock()?.querySelector('p');
const veil = () => document.querySelector('.bg-ui-bg\\/70');
const player = () => document.querySelector('[data-surface="player"]');
const notifications = () => document.querySelector('ul[aria-label="lock.notifications"]');
const field = () => document.getElementById('lock-password');
const avatar = () => document.querySelector('img[src="data:image/png;base64,BBBB"]');

beforeEach(() => {
	forgetEverything();
});

afterEach(() => {
	wrapper?.unmount();
	wrapper = null;
	document.body.innerHTML = '';
});

describe('sin la sección lock la pantalla se ve como siempre', () => {
	for (const [nombre, lock] of [
		['ausente', undefined],
		['vacía', {}],
		['con claves que no existen', { otra: 1 }],
	] as const) {
		test(`con lock ${nombre}: reloj con fecha y tamaño grande, velo, medios y avisos`, async () => {
			await open(lock);

			expect(time().textContent?.trim()).not.toBe('');
			expect(date()).not.toBeNull();
			expect(time().className).toContain('text-display-l');
			expect(veil()).not.toBeNull();
			expect(player()).not.toBeNull();
			expect(notifications()).not.toBeNull();
			expect(field()).not.toBeNull();
		});
	}
});

describe('cada clave cambia lo suyo', () => {
	test('lock.clock.hour12 en true muestra la hora con AM/PM', async () => {
		await open({ clock: { hour12: true } });
		expect(time().textContent).toMatch(/AM|PM/i);
	});

	test('lock.clock.hour12 en false muestra la hora de 24 horas, sin AM/PM', async () => {
		await open({ clock: { hour12: false } });
		expect(time().textContent).not.toMatch(/AM|PM/i);
		expect(time().textContent?.trim()).toMatch(/^\d{2}:\d{2}$/);
	});

	test('lock.clock.date en false quita la fecha y deja la hora', async () => {
		await open({ clock: { date: false } });
		expect(date()).toBeNull();
		expect(time()).not.toBeNull();
	});

	test('lock.clock.size en md achica el reloj', async () => {
		await open({ clock: { size: 'md' } });
		expect(time().className).toContain('text-heading-l');
		expect(time().className).not.toContain('text-display-l');
	});

	test('lock.background.dim en false con imagen propia quita el velo', async () => {
		await open({ background: { source: 'image', image: '/x.png', dim: false } });
		expect(veil()).toBeNull();
		// El fondo sigue ahí: sólo se fue la atenuación.
		expect(document.querySelector(`img[src="${PICTURE}"]`)).not.toBeNull();
	});

	test('lock.background.dim en true con imagen propia deja el velo', async () => {
		await open({ background: { source: 'image', image: '/x.png', dim: true } });
		expect(veil()).not.toBeNull();
	});

	test('dim sólo cuenta con imagen propia: con el fondo del escritorio el velo se queda', async () => {
		await open({ background: { source: 'desktop', dim: false } });
		expect(veil()).not.toBeNull();
	});

	test('lock.media en false oculta la tarjeta aunque esté sonando algo', async () => {
		await open({ media: false });
		expect(player()).toBeNull();
		expect(notifications()).not.toBeNull();
	});

	test('lock.notifications en false oculta los avisos aunque haya notificaciones', async () => {
		await open({ notifications: false });
		expect(notifications()).toBeNull();
		expect(player()).not.toBeNull();
	});
});

describe('un valor inválido cae al de hoy sin romper la pantalla', () => {
	test('size "xl", hour12 "si", date 0, media "no", notifications null y dim "no"', async () => {
		await open({
			clock: { size: 'xl', hour12: 'si', date: 0 },
			background: { source: 'image', dim: 'no' },
			media: 'no',
			notifications: null,
		});

		expect(time().className).toContain('text-display-l');
		expect(date()).not.toBeNull();
		expect(veil()).not.toBeNull();
		expect(player()).not.toBeNull();
		expect(notifications()).not.toBeNull();
		expect(field()).not.toBeNull();
	});

	test('una sección lock que no es un objeto tampoco rompe', async () => {
		for (const lock of ['texto', 7, true, []]) {
			await open(lock);
			expect(field(), `lock = ${JSON.stringify(lock)}`).not.toBeNull();
			expect(player()).not.toBeNull();
			wrapper?.unmount();
			document.body.innerHTML = '';
			forgetEverything();
		}
	});
});

describe('la pantalla nunca se queda sin forma de desbloquear', () => {
	const todoApagado = {
		clock: { hour12: false, date: false, size: 'md' },
		background: { source: 'color', dim: false },
		media: false,
		notifications: false,
	};

	for (const [nombre, lock] of [
		['todo apagado', todoApagado],
		['imagen sin velo', { background: { source: 'image', dim: false } }],
		['sólo medios y avisos apagados', { media: false, notifications: false }],
		['valores inválidos', { clock: { size: 'xl' }, media: 'no', notifications: [] }],
	] as const) {
		test(`con ${nombre} siguen el campo de contraseña, el botón y el avatar`, async () => {
			await open(lock);

			expect(field()).not.toBeNull();
			expect(document.querySelector('button[type="submit"]')).not.toBeNull();
			expect(avatar()).not.toBeNull();
			expect(document.activeElement).toBe(field());
		});
	}
});

describe('useLockConfig', () => {
	function withLock(lock: unknown) {
		setActivePinia(createPinia());
		(useConfigStore() as unknown as { config: unknown }).config = lock === undefined ? null : { lock };
		return useLockConfig();
	}

	test('sin configuración da los valores de hoy', () => {
		const lock = withLock(undefined);
		expect(lock.hour12.value).toBeUndefined();
		expect(lock.showDate.value).toBe(true);
		expect(lock.clockSize.value).toBe('lg');
		expect(lock.backgroundSource.value).toBe('desktop');
		expect(lock.dimBackground.value).toBe(true);
		expect(lock.showMedia.value).toBe(true);
		expect(lock.showNotifications.value).toBe(true);
	});

	test('lee cada clave con su tipo', () => {
		const lock = withLock({
			clock: { hour12: false, date: false, size: 'md' },
			background: { source: 'image', dim: false },
			media: false,
			notifications: false,
		});
		expect(lock.hour12.value).toBe(false);
		expect(lock.showDate.value).toBe(false);
		expect(lock.clockSize.value).toBe('md');
		expect(lock.backgroundSource.value).toBe('image');
		expect(lock.dimBackground.value).toBe(false);
		expect(lock.showMedia.value).toBe(false);
		expect(lock.showNotifications.value).toBe(false);
	});

	test('el origen del fondo desconocido vuelve al escritorio y el color no tiene velo que quitar', () => {
		expect(withLock({ background: { source: 'video' } }).backgroundSource.value).toBe('desktop');
		const color = withLock({ background: { source: 'color', dim: false } });
		expect(color.backgroundSource.value).toBe('color');
		expect(color.dimBackground.value).toBe(true);
	});

	test('un tipo equivocado cae al valor de hoy', () => {
		const lock = withLock({
			clock: { hour12: 'si', date: 'no', size: 'xl' },
			background: { source: 7, dim: 'no' },
			media: 0,
			notifications: 'false',
		});
		expect(lock.hour12.value).toBeUndefined();
		expect(lock.showDate.value).toBe(true);
		expect(lock.clockSize.value).toBe('lg');
		expect(lock.backgroundSource.value).toBe('desktop');
		expect(lock.showMedia.value).toBe(true);
		expect(lock.showNotifications.value).toBe(true);
	});
});
