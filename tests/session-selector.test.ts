/**
 * El selector de sesión del saludo: el `SearchSelect` de la librería sin
 * buscador.
 *
 * Es la lista que se abre con el teclado sin tocar el mouse: flechas, Enter y
 * Escape. Se mira el estado (`aria-expanded`, la sesión elegida) y si la tecla
 * quedó cancelada, no sólo el dibujo.
 */

import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import SessionSelector from '../src/components/SessionSelector.vue';
import { useGreeter } from '../src/composables/useGreeter';
import type { Session } from '../src/types/greeter';
import { forgetEverything } from './doubles';

const session = (id: string, name: string): Session => ({
	id,
	name,
	comment: '',
	exec: id,
	path: '',
	session_type: 'wayland',
	desktop_names: [],
});

const SESSIONS = [session('vasakos', 'VasakOS'), session('gnome', 'GNOME')];

let wrapper: VueWrapper | null = null;

async function open() {
	const greeter = useGreeter();
	greeter.sessions.value = SESSIONS;
	greeter.selectedSession.value = SESSIONS[0] ?? null;
	wrapper = mount(SessionSelector, { attachTo: document.body });
	await flushPromises();
	return wrapper;
}

function key(target: Element, name: string) {
	const event = new KeyboardEvent('keydown', { key: name, bubbles: true, cancelable: true });
	target.dispatchEvent(event);
	return event;
}

const trigger = () => wrapper?.find('button[aria-haspopup="listbox"]').element as HTMLElement;
const listbox = () => document.querySelector<HTMLElement>('[role="listbox"]');
const options = () => [...document.querySelectorAll<HTMLElement>('[role="option"]')];

beforeEach(() => forgetEverything());
afterEach(() => {
	wrapper?.unmount();
	wrapper = null;
	document.body.innerHTML = '';
});

describe('el selector de sesión', () => {
	test('el botón muestra la sesión elegida y se nombra', async () => {
		await open();
		expect(trigger().textContent).toContain('VasakOS');
		expect(trigger().getAttribute('aria-label')).toBe('login.session');
		expect(trigger().getAttribute('aria-expanded')).toBe('false');
	});

	test('sin buscador: abre la lista y el foco va a la lista, con la elegida marcada', async () => {
		await open();
		trigger().focus();
		key(trigger(), 'ArrowDown');
		await flushPromises();

		expect(trigger().getAttribute('aria-expanded')).toBe('true');
		expect(document.querySelector('input')).toBeNull();
		expect(document.activeElement).toBe(listbox());
		expect(options().map((option) => option.getAttribute('aria-selected'))).toEqual(['true', 'false']);
	});

	test('con el teclado: flecha baja y Enter elige la otra sesión', async () => {
		await open();
		key(trigger(), 'ArrowDown');
		await flushPromises();

		key(listbox() as HTMLElement, 'ArrowDown');
		await flushPromises();
		expect(key(listbox() as HTMLElement, 'Enter').defaultPrevented).toBe(true);
		await flushPromises();

		expect(useGreeter().selectedSession.value?.id).toBe('gnome');
		expect(listbox()).toBeNull();
	});

	test('Escape cierra la lista sin cambiar la sesión', async () => {
		await open();
		key(trigger(), 'ArrowDown');
		await flushPromises();
		key(listbox() as HTMLElement, 'ArrowDown');

		const event = key(listbox() as HTMLElement, 'Escape');
		await flushPromises();

		expect(event.defaultPrevented).toBe(true);
		expect(listbox()).toBeNull();
		expect(useGreeter().selectedSession.value?.id).toBe('vasakos');
	});

	test('el clic en una opción la elige', async () => {
		await open();
		await wrapper?.find('button[aria-haspopup="listbox"]').trigger('click');
		await flushPromises();
		options()[1]?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
		await flushPromises();
		expect(useGreeter().selectedSession.value?.id).toBe('gnome');
	});

	test('con una sola sesión no hay nada que elegir', async () => {
		const greeter = useGreeter();
		greeter.sessions.value = [SESSIONS[0] as Session];
		wrapper = mount(SessionSelector, { attachTo: document.body });
		await flushPromises();
		expect(wrapper.find('button').exists()).toBe(false);
		expect(wrapper.text()).toContain('VasakOS');
	});
});
