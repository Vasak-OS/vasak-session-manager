/**
 * Las cuentas del saludo: el `OptionGroup` de la librería con un avatar por
 * cuenta y «Otra cuenta…» al final.
 */

import { afterEach, describe, expect, test } from 'bun:test';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import UserSelector from '../src/components/UserSelector.vue';
import { useGreeter } from '../src/composables/useGreeter';
import type { SystemUser } from '../src/types/greeter';

const user = (name: string, real_name: string): SystemUser => ({
	name,
	real_name,
	uid: name.length,
	gid: 1000,
	home: `/home/${name}`,
	shell: '/bin/zsh',
	avatar: null,
});

let wrapper: VueWrapper | null = null;

async function open() {
	const greeter = useGreeter();
	greeter.users.value = [user('pato', 'Pato'), user('invitado', '')];
	greeter.selectUser(greeter.users.value[0] as SystemUser);
	wrapper = mount(UserSelector, { attachTo: document.body });
	await flushPromises();
	return wrapper;
}

const radios = () => [...document.querySelectorAll<HTMLElement>('[role="radio"]')];

afterEach(() => {
	wrapper?.unmount();
	wrapper = null;
	document.body.innerHTML = '';
});

describe('el selector de cuentas', () => {
	test('una opción por cuenta, más «Otra cuenta…», con la elegida marcada', async () => {
		await open();
		expect(document.querySelector('[role="radiogroup"]')?.getAttribute('aria-label')).toBe('login.selectUser');
		expect(radios().map((radio) => radio.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
			'PPato@pato',
			'Iinvitado@invitado',
			'login.otherUser',
		]);
		expect(radios().map((radio) => radio.getAttribute('aria-checked'))).toEqual(['true', 'false', 'false']);
	});

	test('elegir otra cuenta cambia la cuenta con la que se entra', async () => {
		await open();
		radios()[1]?.click();
		await flushPromises();
		expect(useGreeter().username.value).toBe('invitado');
		expect(useGreeter().usingManualEntry.value).toBe(false);
	});

	test('«Otra cuenta…» pasa a escribir el nombre a mano', async () => {
		await open();
		radios()[2]?.click();
		await flushPromises();
		expect(useGreeter().usingManualEntry.value).toBe(true);
		expect(radios()[2]?.getAttribute('aria-checked')).toBe('true');
	});

	test('las flechas pasan de una cuenta a la siguiente y se llevan el foco', async () => {
		await open();
		const first = radios()[0] as HTMLElement;
		first.focus();
		const event = new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true });
		first.dispatchEvent(event);
		await flushPromises();
		expect(event.defaultPrevented).toBe(true);
		expect(useGreeter().username.value).toBe('invitado');
		expect(document.activeElement).toBe(radios()[1] as HTMLElement);
	});
});
