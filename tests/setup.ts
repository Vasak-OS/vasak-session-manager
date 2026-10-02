/**
 * Lo que tiene que estar listo antes de la primera prueba.
 *
 * Corre como `preload` de `bun test` (ver `bunfig.toml`): el DOM, el compilador
 * de `.vue` y los dobles de Tauri tienen que estar puestos antes de que se
 * importe un componente, porque los componentes los llaman al importarse.
 */

import { mock } from 'bun:test';
import { GlobalRegistrator } from '@happy-dom/global-registrator';
import * as core from '@tauri-apps/api/core';
import * as events from '@tauri-apps/api/event';
import * as icons from '@vasakgroup/plugin-vicons';
import './vue-plugin';
import { convertFileSrc, emit, getIconSource, getSymbolSource, invoke, listen, useI18n } from './doubles';

GlobalRegistrator.register();

/**
 * El objeto que Tauri le inyecta a la ventana. `getCurrentWindow()` lee de acá
 * la etiqueta, y la pantalla de bloqueo decide con ella si dibuja el
 * formulario.
 */
(globalThis as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
	metadata: { currentWindow: { label: 'lock-0' }, currentWebview: { label: 'lock-0', windowLabel: 'lock-0' } },
	invoke,
	convertFileSrc,
	transformCallback: (callback: unknown) => callback,
};

// Los dobles **encima** del módulo de verdad: reemplazarlo entero deja sin
// exportar lo que no se nombra acá, y los componentes de la librería vienen
// compilados e importan de `@tauri-apps/api` cosas internas.
mock.module('@tauri-apps/api/core', () => ({ ...core, invoke, convertFileSrc }));
mock.module('@tauri-apps/api/event', () => ({ ...events, listen, emit }));
mock.module('@vasakgroup/tauri-plugin-i18n', () => ({ default: { getInstance: () => ({ load: async () => {} }) }, useI18n }));
mock.module('@vasakgroup/plugin-vicons', () => ({ ...icons, getIconSource, getSymbolSource }));
