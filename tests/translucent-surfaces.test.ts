/**
 * Las superficies del saludo y de la pantalla de bloqueo: translúcidas **y con
 * desenfoque**.
 *
 * Es la excepción de esta aplicación a la regla del taller (decisión del
 * usuario, 02/10/2026). En el resto del escritorio lo que va sobre el fondo usa
 * `bg-ui-shell` y nunca `backdrop-blur`, porque el desenfoque lo pone Wayfire
 * detrás de la superficie (`vasak-desktop/tests/translucent-surfaces.test.ts`).
 * Acá no hay Wayfire detrás: el saludo corre en `cage` antes de que exista una
 * sesión, y la pantalla de bloqueo es una superficie de `ext-session-lock` que
 * tapa todo. El fondo de pantalla lo dibuja la propia página, así que el
 * desenfoque también: la tarjeta, el campo, los botones de energía y el reloj
 * dejan ver la foto borrosa detrás.
 *
 * Así que esta prueba pide las dos cosas a la vez: que cada superficie deje ver
 * lo de atrás (`ui-shell` o un fondo con opacidad) **y** que lo desenfoque. Lo
 * que flota **dentro** de la tarjeta (la lista de sesiones) sigue opaco, con
 * `ui-float`, como cualquier desplegable del sistema.
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { flushPromises, mount } from '@vue/test-utils';
import LoginInput from '../src/components/LoginInput.vue';
import { useGreeter } from '../src/composables/useGreeter';

const ROOT = join(import.meta.dir, '..');
const read = (file: string) => readFileSync(join(ROOT, file), 'utf8');

/** Saca los comentarios HTML cortando por sus delimitadores. */
function stripHtmlComments(text: string): string {
	let out = '';
	let index = 0;
	while (index < text.length) {
		const start = text.indexOf('<!--', index);
		if (start === -1) return out + text.slice(index);
		out += text.slice(index, start);
		const end = text.indexOf('-->', start + 4);
		if (end === -1) return out;
		index = end + 3;
	}
	return out;
}

const template = (file: string) => {
	const text = read(file);
	return stripHtmlComments(text.slice(text.indexOf('<template>'), text.lastIndexOf('</template>')));
};

/** Las clases del elemento marcado con `data-surface="name"`. */
function surfaceClasses(file: string, name: string): string {
	const tag = template(file).match(new RegExp(`<[a-z]+\\b[^>]*data-surface="${name}"[^>]*>`))?.[0];
	expect(tag, `no se encontró data-surface="${name}" en ${file}`).toBeDefined();
	return tag?.match(/\sclass="([^"]*)"/)?.[1] ?? '';
}

/** Los fondos que nombra un trozo de plantilla, sin variantes de estado. */
function backgroundsOf(classes: string): string[] {
	return [...classes.matchAll(/(?<![\w:/-])!?bg-([a-z][\w-]*(?:\/\d+)?|\[[^\]]+\](?:\/\d+)?)(?![\w/-])/g)].map(
		(match) => match[1] as string
	);
}

/** Un fondo deja ver lo de atrás si es `ui-shell`, `transparent` o lleva `/NN` < 100. */
function isTranslucent(background: string): boolean {
	if (background === 'ui-shell' || background === 'transparent') return true;
	const alpha = background.match(/\/(\d+)$/)?.[1];
	return alpha !== undefined && Number(alpha) < 100;
}

/**
 * Lo que tiene que cumplir una superficie de esta aplicación: un fondo, todos
 * translúcidos, y el desenfoque. `needsBackground: false` es para el marco de
 * un componente de la librería que trae su propio fondo translúcido.
 */
function surfaceProblems(classes: string, needsBackground = true): string[] {
	const problems: string[] = [];
	const backgrounds = backgroundsOf(classes);
	if (needsBackground && backgrounds.length === 0) problems.push(`sin fondo: «${classes}»`);
	for (const background of backgrounds) {
		if (!isTranslucent(background)) problems.push(`opaco: bg-${background}`);
	}
	if (!/(?<![\w:-])backdrop-blur(?:-[a-z0-9]+)?(?![\w-])/.test(classes)) problems.push('sin backdrop-blur');
	return problems;
}

const SURFACES: Array<[string, string, boolean]> = [
	['src/App.vue', 'card', true],
	['src/components/GreeterClock.vue', 'clock', true],
	['src/components/PowerMenu.vue', 'power', true],
	['src/LockView.vue', 'lock-card', true],
	['src/LockView.vue', 'player', true],
	// El recuadro de avisos es un `IconTile`, que trae su fondo translúcido
	// (`bg-ui-surface/70`, comprobado abajo): el marco sólo pone el desenfoque.
	['src/LockView.vue', 'notifications', false],
];

describe('las superficies sobre el fondo de pantalla dejan verlo, desenfocado', () => {
	for (const [file, name, needsBackground] of SURFACES) {
		test(`${name} (${file.replace('src/', '')}) es translúcida y con backdrop-blur`, () => {
			expect(surfaceProblems(surfaceClasses(file, name), needsBackground)).toEqual([]);
		});
	}

	test('la tarjeta y el formulario de bloqueo usan ui-shell, el fondo de las superficies del escritorio', () => {
		expect(backgroundsOf(surfaceClasses('src/App.vue', 'card'))).toEqual(['ui-shell']);
		expect(backgroundsOf(surfaceClasses('src/LockView.vue', 'lock-card'))).toEqual(['ui-shell']);
	});

	test('el campo de la contraseña también deja ver lo de atrás', async () => {
		// Es de la librería: lo que se mira es lo que dibuja de verdad.
		const greeter = useGreeter();
		greeter.selectedSession.value = null;
		const wrapper = mount(LoginInput, { attachTo: document.body });
		await flushPromises();
		const field = wrapper.find('input[type="password"]');
		const backgrounds = backgroundsOf(field.attributes('class') ?? '');
		expect(backgrounds.length).toBeGreaterThan(0);
		expect(backgrounds.every(isTranslucent)).toBe(true);
		wrapper.unmount();
		document.body.innerHTML = '';
	});

	test('el recuadro de avisos de la librería es translúcido', () => {
		const library = read('node_modules/@vasakgroup/vue-libvasak/dist/vue-libvasak.es.js');
		const neutral = library.match(/neutral:\s*"(border-ui-line bg-[^"]+)"/)?.[1] ?? '';
		expect(neutral).not.toBe('');
		expect(backgroundsOf(neutral).every(isTranslucent)).toBe(true);
	});

	test('lo que flota dentro de la tarjeta va opaco: la lista de sesiones', () => {
		const list = template('src/components/SessionSelector.vue').match(/<div\b[^>]*id="session-list"[^>]*>/)?.[0] ?? '';
		expect(backgroundsOf(list)).toEqual(['ui-float']);
		expect(list).not.toMatch(/backdrop-blur/);
	});

	test('ui-shell existe y es translúcida en la librería instalada', () => {
		const tokens = read('node_modules/@vasakgroup/vue-libvasak/dist/tokens.css');
		const shell = tokens.match(/--color-ui-shell:\s*([^;]+);/)?.[1] ?? '';
		expect(shell).toMatch(/^color-mix\(in srgb, var\(--use-ui-background\) (\d+)%, transparent\)$/);
		expect(Number(shell.match(/(\d+)%/)?.[1])).toBeLessThan(100);
	});
});

describe('la guardia de translucidez ve lo que falta cuando falta', () => {
	test('rechaza lo opaco y lo que no desenfoca', () => {
		expect(surfaceProblems('bg-ui-bg border backdrop-blur-md')).toEqual(['opaco: bg-ui-bg']);
		expect(surfaceProblems('bg-ui-shell rounded-corner-xl')).toEqual(['sin backdrop-blur']);
		expect(surfaceProblems('bg-ui-shell !bg-[#fff] backdrop-blur-md')).toEqual(['opaco: bg-[#fff]']);
		expect(surfaceProblems('rounded-corner-xl backdrop-blur-md')).toHaveLength(1);
		// Un desenfoque sólo al pasar el puntero no cuenta.
		expect(surfaceProblems('bg-ui-shell hover:backdrop-blur-md')).toEqual(['sin backdrop-blur']);
	});

	test('y deja pasar las superficies translúcidas con desenfoque', () => {
		expect(surfaceProblems('bg-ui-shell backdrop-blur-md rounded-corner-xl')).toEqual([]);
		expect(surfaceProblems('bg-ui-bg/80 backdrop-blur hover:bg-ui-hover')).toEqual([]);
		expect(surfaceProblems('rounded-corner-m backdrop-blur-md', false)).toEqual([]);
	});
});
