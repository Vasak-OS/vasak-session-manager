/**
 * Compilar los componentes de un solo archivo para `bun test`.
 *
 * Bun no entiende `.vue`: los trata como un archivo suelto y devuelve su ruta
 * como una cadena. Importar un componente «funcionaba» —la importación no
 * fallaba— pero lo que llegaba era texto, y montarlo reventaba adentro de
 * `@vue/test-utils` con un error que no nombra a Vue por ningún lado.
 *
 * Así que se compila acá, con el mismo `@vue/compiler-sfc` que usa Vite, y
 * queda registrado como complemento de Bun desde `tests/setup.ts`. Es la copia
 * del de vasak-terminal (`tests/complemento-vue.ts`), con los nombres en inglés.
 *
 * Se compilan el guión y la plantilla; los estilos se descartan: son Tailwind
 * sobre variables del tema y no llegan a `happy-dom` con nada que aplicarles.
 * Las clases de un elemento se siguen pudiendo comprobar porque están en la
 * plantilla.
 */

import { plugin } from 'bun';
import { compileScript, compileTemplate, parse, rewriteDefault } from 'vue/compiler-sfc';

/** Un identificador estable por archivo: `compileScript` lo pide, y la ruta alcanza. */
function stableId(path: string) {
	return Bun.hash(path).toString(16);
}

plugin({
	name: 'vue',
	setup(build) {
		build.onLoad({ filter: /\.vue$/ }, async ({ path }) => {
			const source = await Bun.file(path).text();
			const { descriptor } = parse(source, { filename: path });
			const id = stableId(path);
			const isTs = (descriptor.scriptSetup?.lang ?? descriptor.script?.lang) === 'ts';
			const plugins: 'typescript'[] = isTs ? ['typescript'] : [];
			const templateOptions = {
				id,
				filename: path,
				compilerOptions: { expressionPlugins: plugins },
			};

			const parts: string[] = [];

			if (descriptor.scriptSetup || descriptor.script) {
				// Con `<script setup>` la plantilla va adentro del propio `setup`,
				// como hace Vite en producción.
				const script = compileScript(descriptor, {
					id,
					inlineTemplate: Boolean(descriptor.scriptSetup),
					templateOptions,
				});
				parts.push(rewriteDefault(script.content, '__sfc__', plugins));
			} else {
				parts.push('const __sfc__ = {};');
			}

			if (descriptor.template && !descriptor.scriptSetup) {
				const template = compileTemplate({ ...templateOptions, source: descriptor.template.content });
				parts.push(template.code, '__sfc__.render = render;');
			}

			parts.push(`__sfc__.__file = ${JSON.stringify(path)};`, 'export default __sfc__;');

			return { contents: parts.join('\n'), loader: 'ts' };
		});
	},
});
