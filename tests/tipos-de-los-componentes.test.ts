/**
 * Que ningún archivo del repositorio aplane los tipos de los componentes.
 *
 * Había un `declare module "*.vue"` en `vite-env.d.ts`, con
 * `DefineComponent<{}, {}, any>`: herencia de cuando el chequeo lo hacía `tsc`
 * a secas. `vue-tsc` entiende los `.vue` de forma nativa y no lo necesita.
 *
 * **Lo que hacía hoy: nada.** Es una declaración *ambiente*, y esas se aplican
 * sólo cuando TypeScript no encuentra un `.d.ts` —o el `.vue` de verdad— para
 * el import. Se comprobó metiendo un error de tipo a propósito con el comodín
 * puesto y sin él: salta en los dos casos.
 *
 * Se saca igual porque es una trampa armada: en cuanto un import no resuelva a
 * un archivo con tipos, el comodín se aplica y el chequeo deja de mirar sin que
 * nada avise. Eso ya pasó en vasak-desktop, con los componentes de
 * `vue-libvasak` en su línea 0.2.
 *
 * Este repositorio se salvó del barrido que sacó el comodín de los otros
 * diecinueve: lo escribía con **comillas dobles**, y los patrones que buscaban
 * el comodín iban con simples. Por eso el de acá los admite a los dos.
 */

import { describe, expect, test } from 'bun:test';
import { fileURLToPath } from 'node:url';

// `fileURLToPath` y no `.pathname`: éste deja los caracteres escapados, así que
// un checkout en una ruta con espacios mandaría a `Bun.Glob` y `Bun.file` a una
// carpeta que no existe.
const raiz = fileURLToPath(new URL('..', import.meta.url));
/** Este mismo archivo, relativo a la raíz. */
const propio = fileURLToPath(import.meta.url).slice(raiz.length);

/**
 * Todo lo que el chequeo de tipos mira, no sólo `src`.
 *
 * El `tsconfig` de la raíz mira `src/**` y el de node los archivos de
 * configuración sueltos; `tests/` se suma porque una declaración ambiente
 * puesta ahí aplanaría los tipos igual en cuanto alguien amplíe el `include`.
 * Con un patrón más angosto las pruebas de abajo pasarían sin haberla visto.
 */
const fuentes = (
	await Promise.all(
		['src/**/*.{ts,tsx,mts,cts,vue}', 'tests/**/*.{ts,tsx,vue}', '*.{ts,mts,cts}'].map(
			async (patron) => await Array.fromAsync(new Bun.Glob(patron).scan({ cwd: raiz })),
		),
	)
).flat()
	// Menos este archivo. Los patrones que busca los lleva escritos adentro,
	// así que al ampliar el escaneo a `tests/` empezó a encontrarse a sí mismo.
	.filter((ruta) => ruta !== propio);

async function conteniendo(patron: RegExp): Promise<string[]> {
	const hallados: string[] = [];
	for (const ruta of fuentes) {
		if (patron.test(await Bun.file(`${raiz}${ruta}`).text())) hallados.push(ruta);
	}
	return hallados.sort();
}

describe('los tipos de los componentes', () => {
	test('y las dos pruebas que siguen miran archivos de verdad', () => {
		// Las dos buscan algo que no tiene que aparecer, así que pasan solas si
		// la lista viene vacía —una `raiz` mal armada y no hay nada que mirar—.
		expect(fuentes).toContain('src/vite-env.d.ts');
		expect(fuentes).toContain('src/main.ts');
		// Y que los tres patrones traigan algo: el de `tests` y el de la raíz se
		// suman porque el de `src` solo deja huecos, y un patrón que no
		// encuentra nada los deja igual.
		expect(fuentes.some((ruta) => ruta.startsWith('tests/'))).toBe(true);
		expect(fuentes).toContain('vite.config.ts');
		expect(fuentes.length).toBeGreaterThan(3);
	});

	test('no los aplana ningún comodín de .vue', async () => {
		// Las dos comillas: con simples se escribía en los otros repositorios,
		// con dobles acá, y el barrido que buscaba sólo las simples pasó de
		// largo por este archivo.
		expect(await conteniendo(/declare\s+module\s+['"]\*\.vue['"]/)).toEqual([]);
	});

	test('ni los redeclara a mano ningún paquete del ecosistema', async () => {
		// Un `declare module` de un paquete instalado gana siempre, así que lo
		// que diga ese archivo es lo único que se comprueba. Pasó con
		// `@vasakgroup/vue-libvasak` en la galería y en el escritorio.
		expect(await conteniendo(/declare\s+module\s+['"]@vasakgroup\//)).toEqual([]);
	});
});
