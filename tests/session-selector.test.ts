/**
 * El selector de sesión del saludo, con las filas de la librería.
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

const trigger = () => wrapper?.find('[role="combobox"]').element as HTMLElement;

beforeEach(() => forgetEverything());
afterEach(() => {
	wrapper?.unmount();
	wrapper = null;
	document.body.innerHTML = '';
});

describe('el selector de sesión', () => {
	test('las opciones son filas de la librería con su estado', async () => {
		await open();
		const options = wrapper?.findAll('[role="option"]') ?? [];
		expect(options.map((option) => option.text())).toEqual(['VasakOS', 'GNOME']);
		expect(options.map((option) => option.attributes('aria-selected'))).toEqual(['true', 'false']);
	});

	test('con el teclado: flecha abre, flecha baja, Enter elige y cierra', async () => {
		await open();
		trigger().focus();

		expect(key(trigger(), 'ArrowDown').defaultPrevented).toBe(true);
		await flushPromises();
		expect(trigger().getAttribute('aria-expanded')).toBe('true');

		key(trigger(), 'ArrowDown');
		await flushPromises();
		expect(trigger().getAttribute('aria-activedescendant')).toBe('session-option-1');

		expect(key(trigger(), 'Enter').defaultPrevented).toBe(true);
		await flushPromises();
		expect(useGreeter().selectedSession.value?.id).toBe('gnome');
		expect(trigger().getAttribute('aria-expanded')).toBe('false');
		expect(document.activeElement).toBe(trigger());
	});

	test('Escape cierra la lista abierta sin cambiar la sesión', async () => {
		await open();
		trigger().focus();
		key(trigger(), 'ArrowDown');
		await flushPromises();
		key(trigger(), 'ArrowDown');

		const event = key(trigger(), 'Escape');
		await flushPromises();

		expect(event.defaultPrevented).toBe(true);
		expect(trigger().getAttribute('aria-expanded')).toBe('false');
		expect(useGreeter().selectedSession.value?.id).toBe('vasakos');
		expect(document.activeElement).toBe(trigger());
	});

	test('Escape con la lista cerrada no se lo queda: sigue para quien lo escuche', async () => {
		await open();
		expect(key(trigger(), 'Escape').defaultPrevented).toBe(false);
	});

	test('el clic en una opción la elige', async () => {
		await open();
		await wrapper?.find('[role="combobox"]').trigger('click');
		await wrapper?.findAll('[role="option"]')[1]?.trigger('click');
		expect(useGreeter().selectedSession.value?.id).toBe('gnome');
	});

	test('la lista flota opaca dentro de la tarjeta, como cualquier desplegable', async () => {
		await open();
		expect(wrapper?.find('[role="listbox"]').classes()).toContain('bg-ui-float');
	});
});
