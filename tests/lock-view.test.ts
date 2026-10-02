/**
 * La pantalla de bloqueo: si el campo, el Enter, el foco o los errores fallan,
 * no se puede volver a entrar a la sesión.
 *
 * Todo contra los dobles (`tests/doubles.ts`): ningún `unlock` llega a PAM.
 * Igual que en `login-form.test.ts`, se mira adónde fue el foco y si el evento
 * quedó cancelado, nunca «el foco no se fue» (memoria
 * `el-tab-despachado-a-mano-no-navega`).
 */

import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import { createPinia } from 'pinia';
import LockView from '../src/LockView.vue';
import { callsOf, forgetEverything, respond } from './doubles';

let wrapper: VueWrapper | null = null;

async function open() {
	respond('lock_user', 'pato');
	respond('lock_avatar', null);
	respond('lock_background', null);
	// Esta pantalla (`lock-0`, la del doble de la ventana) es la que dibuja.
	respond('lock_active_screen', 'lock-0');
	wrapper = mount(LockView, { attachTo: document.body, global: { plugins: [createPinia()] } });
	await flushPromises();
	return wrapper;
}

const field = () => document.getElementById('lock-password') as HTMLInputElement;

async function type(value: string) {
	field().value = value;
	field().dispatchEvent(new Event('input', { bubbles: true }));
	await flushPromises();
}

function key(target: Element, name: string) {
	const event = new KeyboardEvent('keydown', { key: name, bubbles: true, cancelable: true });
	target.dispatchEvent(event);
	return event;
}

async function submit() {
	wrapper?.find('form').element.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	await flushPromises();
}

beforeEach(() => {
	forgetEverything();
});

afterEach(() => {
	wrapper?.unmount();
	wrapper = null;
	document.body.innerHTML = '';
});

describe('el foco de la pantalla de bloqueo', () => {
	test('al abrir está en el campo de la contraseña', async () => {
		await open();
		expect(field()).not.toBeNull();
		expect(document.activeElement).toBe(field());
	});

	test('una contraseña rechazada deja el campo vacío, activo y con el foco', async () => {
		respond('unlock', false);
		await open();
		await type('incorrecta');
		(wrapper?.find('button[type="submit"]').element as HTMLElement).focus();

		await submit();

		expect(field().value).toBe('');
		expect(field().disabled).toBe(false);
		// Antes el foco se pedía con el campo todavía apagado y no llegaba.
		expect(document.activeElement).toBe(field());
	});
});

describe('desbloquear', () => {
	test('se envía una sola vez aunque Enter se repita mientras verifica', async () => {
		let release: (value: boolean) => void = () => {};
		respond('unlock', () => new Promise<boolean>((resolve) => (release = resolve)));
		await open();
		await type('secreta');

		await submit();
		await submit();

		expect(callsOf('unlock')).toHaveLength(1);
		expect(callsOf('unlock')[0]?.args).toEqual({ password: 'secreta' });
		release(false);
		await flushPromises();
	});

	test('vacío no envía, y el botón está apagado', async () => {
		await open();
		await submit();

		expect(callsOf('unlock')).toHaveLength(0);
		const button = wrapper?.find('button[type="submit"]').element as HTMLButtonElement;
		expect(button.disabled).toBe(true);
		expect(document.activeElement).toBe(field());
	});

	test('con algo escrito el botón de envío está activo, que es lo que hace que Enter envíe', async () => {
		await open();
		await type('x');
		const button = wrapper?.find('button[type="submit"]').element as HTMLButtonElement;
		expect(button.disabled).toBe(false);
		expect(field().form).toBe(wrapper?.find('form').element as HTMLFormElement);
	});
});

describe('los errores se ven y se anuncian', () => {
	test('la contraseña incorrecta aparece en un aviso con role="alert" que describe al campo', async () => {
		respond('unlock', false);
		await open();
		await type('incorrecta');
		await submit();

		const alert = wrapper?.find('[role="alert"]');
		expect(alert?.exists()).toBe(true);
		expect(alert?.text()).toContain('lock.wrongPassword');
		expect(field().getAttribute('aria-invalid')).toBe('true');
		expect(field().getAttribute('aria-describedby')).toContain('lock-error');
	});

	test('un fallo del puente también se anuncia', async () => {
		respond('unlock', () => {
			throw new Error('sin respuesta');
		});
		await open();
		await type('secreta');
		await submit();

		expect(wrapper?.find('[role="alert"]').text()).toContain('lock.error');
		expect(document.activeElement).toBe(field());
	});
});

describe('el teclado', () => {
	test('Escape borra lo escrito, no envía y deja el foco en el campo', async () => {
		await open();
		await type('a medias');

		const event = key(field(), 'Escape');
		await flushPromises();

		expect(event.defaultPrevented).toBe(true);
		expect(field().value).toBe('');
		expect(callsOf('unlock')).toHaveLength(0);
		expect(document.activeElement).toBe(field());
	});

	test('no hay trampa de foco: el Tab del campo queda para el navegador', async () => {
		await open();
		await type('x');

		const event = key(field(), 'Tab');

		expect(event.defaultPrevented).toBe(false);
		const focusables = [...document.querySelectorAll<HTMLElement>('input, button, [tabindex]')].filter(
			(element) => !(element as HTMLButtonElement).disabled && element.tabIndex >= 0
		);
		expect(focusables[focusables.indexOf(field()) + 1]?.getAttribute('type')).toBe('submit');
	});
});

describe('lo que cuenta de la sesión', () => {
	test('una respuesta vacía de avisos o del reproductor no se lleva el formulario', async () => {
		// Con `null` la plantilla leía `.length` de nada y la pantalla quedaba
		// sin dibujar: sin campo donde escribir la contraseña.
		respond('lock_notifications', null);
		respond('lock_media', null);
		await open();
		expect(field()).not.toBeNull();
		expect(document.activeElement).toBe(field());
	});

	test('los avisos llegan con los nombres que manda Rust y se nombran enteros', async () => {
		respond('lock_notifications', [{ icon: 'telegram', app: 'Telegram', count: 3 }]);
		await open();

		const tile = wrapper?.find('[role="img"]');
		// El `t()` de los dobles devuelve la clave: lo que importa es que se
		// pidió la de «varias» y que el recuadro tiene nombre.
		expect(tile?.attributes('aria-label')).toBe('lock.notificationsMany');
		expect(wrapper?.text()).toContain('3');
		expect(wrapper?.find('img[src="icon:telegram"]').exists()).toBe(true);
	});

	test('los botones del reproductor le hablan al reproductor que suena', async () => {
		respond('lock_media', { player: 'org.mpris.MediaPlayer2.x', title: 'Tema', artist: 'Alguien', playing: true });
		await open();

		expect(wrapper?.text()).toContain('Tema');
		await wrapper?.find('button[aria-label="lock.pause"]').trigger('click');
		await flushPromises();

		expect(callsOf('lock_media_action')[0]?.args).toEqual({ player: 'org.mpris.MediaPlayer2.x', action: 'playpause' });
	});
});
