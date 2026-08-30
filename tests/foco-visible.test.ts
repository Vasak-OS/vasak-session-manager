import { describe, expect, test } from 'bun:test';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Que siempre se vea dónde está el foco del teclado.
 *
 * Esta es la pantalla donde alguien escribe su contraseña, y muchas veces la usa
 * sin tocar el mouse: si no se ve qué campo está enfocado, no hay forma de saber
 * dónde va a caer lo que se escriba.
 *
 * El indicador propio del escritorio es `focus:ring-*`, que Tailwind implementa
 * como `box-shadow`. Y ahí está el problema de acompañarlo con
 * `focus:outline-none`: **el modo de alto contraste descarta las sombras**, así
 * que en esa configuración no queda ni el anillo ni el contorno del navegador, y
 * el foco se vuelve invisible.
 *
 * Sin `outline-none` los dos conviven, y no se pierde nada: los navegadores
 * dibujan su contorno sólo en `:focus-visible` —es decir, con el teclado—, así
 * que quien usa el mouse ve exactamente lo mismo que antes.
 */
const PROHIBIDO = 'outline-none';

function vistas(directorio: string): string[] {
	return readdirSync(directorio).flatMap((entrada) => {
		const ruta = join(directorio, entrada);
		if (statSync(ruta).isDirectory()) return vistas(ruta);
		return ruta.endsWith('.vue') ? [ruta] : [];
	});
}

describe('el foco del teclado siempre se ve', () => {
	test('ninguna vista apaga el contorno del navegador', () => {
		const culpables = vistas('src').filter((ruta) =>
			readFileSync(ruta, 'utf8').includes(PROHIBIDO),
		);

		expect(culpables).toEqual([]);
	});

	test('y la lista de vistas que se revisa no está vacía', () => {
		// Si `vistas()` dejara de encontrar archivos, la prueba de arriba pasaría
		// sin revisar nada y nadie se enteraría.
		expect(vistas('src').length).toBeGreaterThan(5);
	});
});
