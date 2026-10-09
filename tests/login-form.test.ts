/**
 * El formulario del saludo: lo que decide si alguien entra a su sesión.
 *
 * Todo contra los dobles (`tests/doubles.ts`): ningún `login` llega a greetd.
 * Cada prueba mira **adónde fue el foco** (`document.activeElement`) y, para el
 * teclado, si el evento quedó cancelado (`defaultPrevented`), nunca «el foco no
 * se fue»: un Tab despachado a mano no mueve el foco, así que esa forma pasa
 * siempre (memoria `el-tab-despachado-a-mano-no-navega`).
 */

import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import LoginInput from '../src/components/LoginInput.vue';
import { useGreeter } from '../src/composables/useGreeter';
import type { Session, SystemUser } from '../src/types/greeter';
import { callsOf, forgetEverything, respond } from './doubles';

const SESSION: Session = {
	id: 'vasakos',
	name: 'VasakOS',
	comment: '',
	exec: 'vasak-session',
	path: '',
	session_type: 'wayland',
	desktop_names: ['VasakOS'],
};

const USER: SystemUser = {
	name: 'pato',
	real_name: 'Pato',
	uid: 1000,
	gid: 1000,
	home: '/home/pato',
	shell: '/bin/zsh',
	avatar: null,
};

let wrapper: VueWrapper | null = null;

async function open(options: { manual?: boolean } = {}) {
	const greeter = useGreeter();
	greeter.sessions.value = [SESSION];
	greeter.selectedSession.value = SESSION;
	greeter.manualUsername.value = '';
	if (options.manual) {
		greeter.useManualEntry();
	} else {
		greeter.selectUser(USER);
	}
	wrapper = mount(LoginInput, { attachTo: document.body });
	await flushPromises();
	return wrapper;
}

const passwordField = () => document.getElementById('password-field') as HTMLInputElement;

async function type(value: string) {
	const field = passwordField();
	field.value = value;
	field.dispatchEvent(new Event('input', { bubbles: true }));
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

describe('el foco del saludo', () => {
	test('al abrir está en el campo de la contraseña', async () => {
		await open();
		expect(document.activeElement).toBe(passwordField());
	});

	test('con «Otra cuenta…» está en el nombre, que es lo primero que hay que escribir', async () => {
		await open({ manual: true });
		expect(document.activeElement?.id).toBe('username-field');
	});

	test('después de un error vuelve al campo de la contraseña, vacío', async () => {
		respond('login', () => {
			throw 'Autenticación fallida';
		});
		await open();
		await type('incorrecta');
		// El foco se va a otro lado (el botón, por ejemplo) antes del error.
		(wrapper?.find('button[type="submit"]').element as HTMLElement).focus();

		await submit();

		expect(passwordField().value).toBe('');
		expect(document.activeElement).toBe(passwordField());
	});
});

describe('enviar', () => {
	test('Enter en el campo envía el formulario, y una sola vez aunque se repita', async () => {
		let release: () => void = () => {};
		respond('login', () => new Promise<void>((resolve) => (release = resolve)));
		await open();
		await type('secreta');

		// Dos envíos mientras greetd todavía no contestó: el segundo Enter de
		// alguien impaciente.
		await submit();
		await submit();

		expect(callsOf('login')).toHaveLength(1);
		expect(callsOf('login')[0]?.args).toMatchObject({ username: 'pato', password: 'secreta', cmd: 'vasak-session' });
		release();
	});

	test('el botón de entrar es el que envía: es de tipo submit y está activo', async () => {
		// Lo que hace que Enter en el campo envíe es que el formulario tenga un
		// botón de envío activo (envío implícito de HTML).
		await open();
		const button = wrapper?.find('button[type="submit"]');
		expect(button?.exists()).toBe(true);
		expect((button?.element as HTMLButtonElement).disabled).toBe(false);
		expect(passwordField().form).toBe(wrapper?.find('form').element as HTMLFormElement);
	});

	test('vacío no envía: avisa, y el foco queda en el campo', async () => {
		await open();
		await submit();

		expect(callsOf('login')).toHaveLength(0);
		expect(wrapper?.find('[role="alert"]').text()).toContain('login.passwordRequired');
		expect(document.activeElement).toBe(passwordField());
	});

	test('sin nombre de cuenta no envía y el foco va al nombre', async () => {
		await open({ manual: true });
		await type('secreta');
		await submit();

		expect(callsOf('login')).toHaveLength(0);
		expect(document.activeElement?.id).toBe('username-field');
	});
});

describe('los errores se ven y se anuncian', () => {
	test('el rechazo de greetd aparece en un aviso con role="alert"', async () => {
		respond('login', () => {
			throw 'Autenticación fallida';
		});
		await open();
		await type('incorrecta');
		await submit();

		const alert = wrapper?.find('[role="alert"]');
		expect(alert?.exists()).toBe(true);
		expect(alert?.text()).toContain('Autenticación fallida');
		// Y el campo dice que está mal y por qué: el aviso es su descripción.
		expect(passwordField().getAttribute('aria-invalid')).toBe('true');
		expect(passwordField().getAttribute('aria-describedby')).toContain('login-error');
		expect(document.getElementById('login-error')?.textContent).toContain('Autenticación fallida');
	});
});

describe('el teclado', () => {
	test('el botón de mostrar deja ver lo escrito y lo vuelve a tapar', async () => {
		await open();
		await type('secreta');
		const reveal = document.querySelector<HTMLButtonElement>('[aria-controls="password-field"]');
		reveal?.click();
		await flushPromises();
		expect(passwordField().type).toBe('text');
		expect(reveal?.getAttribute('aria-pressed')).toBe('true');
		reveal?.click();
		await flushPromises();
		expect(passwordField().type).toBe('password');
	});

	test('con Bloq Mayús el campo lo avisa con el texto de la app, atado al campo', async () => {
		await open();
		const event = new KeyboardEvent('keydown', { key: 'A', bubbles: true, cancelable: true });
		Object.defineProperty(event, 'getModifierState', { value: (name: string) => name === 'CapsLock' });
		passwordField().dispatchEvent(event);
		await flushPromises();

		const hint = document.querySelector('[data-caps-lock]');
		expect(hint?.textContent).toContain('login.capsLock');
		expect(passwordField().getAttribute('aria-describedby')).toContain(hint?.id ?? '-');
	});

	test('Escape mientras un método de entrada compone no borra lo escrito', async () => {
		await open();
		await type('casi');
		const event = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true, isComposing: true });
		passwordField().dispatchEvent(event);
		await flushPromises();
		expect(event.defaultPrevented).toBe(false);
		expect(passwordField().value).toBe('casi');
	});

	test('Escape borra lo escrito, no envía y deja el foco en el campo', async () => {
		await open();
		await type('a medias');
		passwordField().focus();

		const event = key(passwordField(), 'Escape');
		await flushPromises();

		expect(event.defaultPrevented).toBe(true);
		expect(passwordField().value).toBe('');
		expect(callsOf('login')).toHaveLength(0);
		expect(document.activeElement).toBe(passwordField());
	});

	test('no hay trampa de foco: el Tab del campo queda para el navegador', async () => {
		await open();
		passwordField().focus();

		const event = key(passwordField(), 'Tab');

		// Nadie se queda con el Tab: el navegador lo lleva al siguiente.
		expect(event.defaultPrevented).toBe(false);
		// Y lo que sigue se puede alcanzar: el botón de mostrar la contraseña y
		// después el de entrar.
		const focusables = [...document.querySelectorAll<HTMLElement>('input, button, [tabindex]')].filter(
			(element) => !(element as HTMLButtonElement).disabled && element.tabIndex >= 0
		);
		const next = focusables.slice(focusables.indexOf(passwordField()) + 1);
		expect(next[0]?.getAttribute('aria-controls')).toBe('password-field');
		expect(next[1]?.getAttribute('type')).toBe('submit');
	});
});
