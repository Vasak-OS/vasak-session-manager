import { sanearUrl } from '@/tools/csp';
import { createApp } from "vue";
import { createPinia } from "pinia";
import I18n from "@vasakgroup/tauri-plugin-i18n";
import App from "./App.vue";
import LockView from "./LockView.vue";
import "./style.css";
import { loadAppearance } from "@/composables/useAppearance";

// Una violación de CSP no se ve: el recurso no carga y la interfaz queda a
// medias sin decir nada. Se sanean **las dos** URLs, porque `sourceFile` también
// puede llevar query con datos sensibles.
document.addEventListener('securitypolicyviolation', (evento) => {
	// El respaldo va **después** de sanear, no antes.
	//
	// Mirando el valor crudo, una entrada como `?token=X` es verdadera y
	// pasa el respaldo de largo — pero lo que queda de ella al sanearla es
	// nada, así que el registro salía con el campo en blanco. Sanear
	// primero y decidir después es lo que hace que un aviso incompleto no
	// exista.
	const recurso = sanearUrl(evento.blockedURI) || '(en línea)';
	const origen = sanearUrl(evento.sourceFile) || 'documento';
	console.error(
		`[CSP] bloqueado ${recurso} por la directiva ` +
			`«${evento.violatedDirective}» en ${origen}:${evento.lineNumber}`
	);
});

// Two screens, one bundle: the greeter and the lock screen are the same
// interface over different moments — one before there is a session, one over a
// session that already exists. The window URL is what says which is being
// drawn.
const isLock = window.location.hash.startsWith("#/lock");

// The greeter runs before any user session/theme is available; default to the
// VasakOS dark scheme (typical for a login screen). The lock screen has a
// configuration to read, and the config store applies it over this.
document.documentElement.classList.add("dark");

// El greeter no tiene «~» donde leer la configuración del usuario, así que sus
// colores salen de /etc, donde los deja la aplicación de configuración. Sin
// esperar: son colores, y hacerlos esperar retrasaría la pantalla en la que hay
// que escribir la contraseña. La pantalla de bloqueo no pasa por acá porque sí
// tiene la configuración del usuario, que es la que corresponde.
if (!isLock) void loadAppearance();

const app = createApp(isLock ? LockView : App);
app.use(createPinia());

// Cargar traducciones con reintentos para mejor experiencia de usuario.
// Un intento que falla se reintenta, pero la espera total sigue acotada: un
// backend colgado tiene que dar una pantalla con las claves a la vista, no una
// pantalla en blanco para siempre.
async function cargarTraducciones(): Promise<void> {
	const MAX_INTENTOS = 3;
	const ESPERA_BASE_MS = 500;
	const ESPERA_MAX_MS = 2000;
	const PLAZO_TOTAL_MS = 3000;

	const intentar = async () => {
		for (let intento = 0; intento < MAX_INTENTOS; intento++) {
			try {
				await I18n.getInstance().load();
				return;
			} catch (error) {
				console.error(
					`No se pudieron cargar las traducciones (intento ${intento + 1}/${MAX_INTENTOS}):`,
					error
				);
				if (intento === MAX_INTENTOS - 1) return;
				const espera = Math.min(ESPERA_BASE_MS * 2 ** intento, ESPERA_MAX_MS);
				await new Promise((resolve) => setTimeout(resolve, espera));
			}
		}
	};

	await Promise.race([
		intentar(),
		new Promise((resolve) => setTimeout(resolve, PLAZO_TOTAL_MS)),
	]);
}

await cargarTraducciones();

app.mount("#app");
