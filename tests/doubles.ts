/**
 * Los dobles de todo lo que sólo existe adentro de la ventana de Tauri.
 *
 * Una prueba montada corre en `happy-dom`: no hay backend que atienda un
 * `invoke`, no hay quien emita un evento y no hay traducciones. Los dobles
 * guardan lo que se les pidió y dejan contestar, que es lo que permite
 * comprobar comportamiento y no sólo forma.
 *
 * Nunca hablan con greetd, PAM ni la sesión de verdad: lo que «contesta» el
 * inicio de sesión o el desbloqueo es lo que la prueba pone con `respond`.
 *
 * Se registran como módulos desde `tests/setup.ts`, una sola vez (memoria
 * `un-solo-mock-por-modulo`). El estado vive acá, así que una prueba que
 * importe este archivo ve lo mismo que vio el componente.
 */

/** Una llamada al backend, tal como la hizo la página. */
export interface Call {
	command: string;
	args: Record<string, unknown>;
}

/** Todo lo que se le pidió al backend, en orden. */
export const calls: Call[] = [];

type Responder = (args: Record<string, unknown>) => unknown;

const responses = new Map<string, Responder>();
const listeners = new Map<string, Set<(event: { payload: unknown }) => unknown>>();

/** Qué contesta el backend a un comando: un valor, o una función que lo arma. */
export function respond(command: string, answer: Responder | unknown) {
	responses.set(command, typeof answer === 'function' ? (answer as Responder) : () => answer);
}

/** Las veces que se pidió un comando. */
export function callsOf(command: string) {
	return calls.filter((call) => call.command === command);
}

export async function invoke(command: string, args: Record<string, unknown> = {}) {
	calls.push({ command, args });
	const responder = responses.get(command);
	// Un comando sin respuesta preparada devuelve nada en vez de romper: son los
	// de alrededor que la prueba no mira.
	return responder ? await responder(args) : undefined;
}

export function convertFileSrc(path: string) {
	return `asset://localhost/${encodeURIComponent(path)}`;
}

export async function listen(name: string, handler: (event: { payload: unknown }) => unknown) {
	const own = listeners.get(name) ?? new Set();
	own.add(handler);
	listeners.set(name, own);
	return () => {
		own.delete(handler);
	};
}

export async function emit(name: string, payload?: unknown) {
	for (const handler of [...(listeners.get(name) ?? [])]) {
		await handler({ payload });
	}
}

/** El `t()` devuelve la clave: una prueba que mire el texto mira la clave. */
export function useI18n() {
	return {
		t: (key: string) => key,
		locale: { value: 'es' },
		setLocale: async () => {},
		availableLocales: { value: ['es'] },
		isLoaded: { value: true },
		reload: async () => {},
	};
}

/** El tema de iconos devuelve el nombre con el tipo: así se ve cuál se pidió. */
export async function getIconSource(name: string) {
	return `icon:${name}`;
}

export async function getSymbolSource(name: string) {
	return `symbol:${name}`;
}

/** Deja los dobles como recién puestos. Va en el `beforeEach` de cada prueba. */
export function forgetEverything() {
	calls.length = 0;
	responses.clear();
	listeners.clear();
}
