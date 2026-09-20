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
 * el comodín iban con simples. Por eso el de acá los admite a los dos, y por
 * eso cada patrón se prueba además contra un caso que **sí** tiene que
 * encontrar: un patrón que no matchea nada deja las pruebas de ausencia en
 * verde, que es exactamente como este archivo pasó desapercibido. Lo marcó la
 * revisión.
 */

import { describe, expect, test } from 'bun:test';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

/** Un `declare module '*.vue'`, con cualquiera de las dos comillas. */
const COMODIN = /declare\s+module\s+['"]\*\.vue['"]/;
/** Un `declare module` a mano de un paquete del ecosistema. */
const PAQUETE = /declare\s+module\s+['"]@vasakgroup\//;

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
async function escanear(cwd: string): Promise<string[]> {
	const listas = await Promise.all(
		['src/**/*.{ts,tsx,mts,cts,vue}', 'tests/**/*.{ts,tsx,vue}', '*.{ts,mts,cts}'].map(
			async (patron) => await Array.fromAsync(new Bun.Glob(patron).scan({ cwd })),
		),
	);
	return listas.flat();
}

const fuentes = (await escanear(raiz))
	// Menos este archivo. Los patrones que busca los lleva escritos adentro,
	// así que al ampliar el escaneo a `tests/` empezó a encontrarse a sí mismo.
	.filter((ruta) => ruta !== propio);

async function conteniendo(
	patron: RegExp,
	archivos: string[] = fuentes,
	base: string = raiz,
): Promise<string[]> {
	const hallados: string[] = [];
	for (const ruta of archivos) {
		if (patron.test(await Bun.file(`${base}${ruta}`).text())) hallados.push(ruta);
	}
	return hallados.sort();
}

describe('los tipos de los componentes', () => {
	test('las pruebas de ausencia miran archivos de verdad', () => {
		// Buscan algo que no tiene que aparecer, así que pasan solas si la lista
		// viene vacía —una `raiz` mal armada y no hay nada que mirar—.
		expect(fuentes).toContain('src/vite-env.d.ts');
		expect(fuentes).toContain('src/main.ts');
		// Y que los tres patrones traigan algo: el de `tests` y el de la raíz se
		// suman porque el de `src` solo deja huecos, y un patrón que no
		// encuentra nada los deja igual.
		expect(fuentes.some((ruta) => ruta.startsWith('tests/'))).toBe(true);
		expect(fuentes).toContain('vite.config.ts');
		expect(fuentes.length).toBeGreaterThan(3);
	});

	test('y los dos patrones encuentran lo que buscan', async () => {
		// El caso positivo, sobre un repositorio de mentira armado aparte. Sin
		// esto, un patrón que dejó de matchear —o un `conteniendo` que siempre
		// devuelve `[]`— deja las dos pruebas de abajo en verde sin haber
		// mirado nada. Es la forma exacta en que este repositorio se salvó del
		// barrido: el patrón buscaba comillas simples y acá había dobles.
		const falso = `${tmpdir()}/vsk-guardia-${Bun.randomUUIDv7()}/`;
		try {
			// Las dos comillas en el comodín, una en cada archivo.
			await Bun.write(`${falso}src/dobles.d.ts`, 'declare module "*.vue" {}\n');
			await Bun.write(`${falso}src/simples.d.ts`, "declare module '*.vue' {}\n");
			await Bun.write(
				`${falso}src/paquete.d.ts`,
				"declare module '@vasakgroup/vue-libvasak' {}\n",
			);
			await Bun.write(`${falso}src/inocente.ts`, 'export const nada = 1;\n');

			const archivos = await escanear(falso);
			expect(archivos.length).toBe(4);

			expect(await conteniendo(COMODIN, archivos, falso)).toEqual([
				'src/dobles.d.ts',
				'src/simples.d.ts',
			]);
			expect(await conteniendo(PAQUETE, archivos, falso)).toEqual(['src/paquete.d.ts']);
		} finally {
			await Bun.$`rm -rf ${falso}`.quiet();
		}
	});

	test('no los aplana ningún comodín de .vue', async () => {
		expect(await conteniendo(COMODIN)).toEqual([]);
	});

	test('ni los redeclara a mano ningún paquete del ecosistema', async () => {
		// Un `declare module` de un paquete instalado gana siempre, así que lo
		// que diga ese archivo es lo único que se comprueba. Pasó con
		// `@vasakgroup/vue-libvasak` en la galería y en el escritorio.
		expect(await conteniendo(PAQUETE)).toEqual([]);
	});
});
