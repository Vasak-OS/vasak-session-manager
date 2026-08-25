import { createApp } from "vue";
import { createPinia } from "pinia";
import I18n from "@vasakgroup/tauri-plugin-i18n";
import App from "./App.vue";
import LockView from "./LockView.vue";
import "./style.css";
import { loadAppearance } from "@/composables/useAppearance";

/**
 * Saca de una URL lo que no debería quedar en un registro.
 *
 * Se conserva el esquema y la autoridad usando `href`, y no `origin +
 * pathname`: para esquemas propios como `asset:` o `ipc:` el `origin` es la
 * cadena «null», así que esa forma escribía `null/ruta` y perdía justamente lo
 * que permite entender qué se bloqueó.
 */
const sanearUrl = (valor: string | null | undefined): string => {
	if (!valor) {
		return '(en línea)';
	}
	try {
		const url = new URL(valor);
		if (url.protocol === 'data:') {
			return 'data:(recortado)';
		}
		// Credenciales, query y fragmento: ahí es donde viajan los tokens.
		url.username = '';
		url.password = '';
		url.search = '';
		url.hash = '';
		return url.href;
	} catch {
		// No era una URL absoluta —'inline', 'eval', una ruta relativa—: tal cual.
		return valor;
	}
};

// Una violación de CSP no se ve: el recurso no carga y la interfaz queda a
// medias sin decir nada. Se sanean **las dos** URLs, porque `sourceFile`
// también puede llevar query con datos sensibles.
document.addEventListener('securitypolicyviolation', (evento) => {
	console.error(
		`[CSP] bloqueado ${sanearUrl(evento.blockedURI)} por la directiva ` +
			`«${evento.violatedDirective}» en ${sanearUrl(evento.sourceFile) || 'documento'}:${evento.lineNumber}`
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

I18n.getInstance().load();

app.mount("#app");
