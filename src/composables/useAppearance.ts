import { invoke } from "@tauri-apps/api/core";

/**
 * Los colores del inicio de sesión.
 *
 * El resto de las aplicaciones de VasakOS toman el esquema del plugin de
 * configuración, que lee `~/.config/vasak`. Acá no hay «~»: el greeter corre sin
 * sesión, así que los colores llegan de dos archivos en `/etc` que escribe la
 * aplicación de configuración, y las variables CSS se aplican con **el mismo
 * mapeo** que usa el plugin. Si los dos mapeos se separan, el inicio de sesión
 * se ve distinto del escritorio con el mismo esquema elegido.
 *
 * Todo esto es «si se puede»: sin archivos, o con un archivo roto, queda la
 * paleta oscura que trae compilada `style.css`. Es una pantalla de la que no se
 * puede salir; quedarse sin colores no puede impedir entrar.
 */
interface Appearance {
	theme: string;
	scheme: SchemeDocument | null;
}

interface SchemeVariant {
	ui: {
		color: { primary: string; secondary: string };
		text: { main: string; muted: string; "on-primary": string };
		background: string;
		border: string;
		surface: string;
	};
	terminal?: { ansi?: Record<string, string> };
}

interface SchemeDocument {
	colors: { dark: SchemeVariant; light: SchemeVariant };
}

/**
 * Qué variable recibe qué color, en un solo lugar.
 *
 * El sufijo `-dark` no es «el tema oscuro está activo»: las dos paletas viven a
 * la vez en `:root` y la clase `dark` del elemento raíz elige cuál se usa. Por
 * eso se escriben las dos, siempre.
 */
const asignaciones = (
	variante: SchemeVariant,
	sufijo: "" | "-dark"
): Array<[string, string | undefined]> => [
	[`--primary${sufijo}`, variante.ui.color.primary],
	[`--secondary${sufijo}`, variante.ui.color.secondary],
	[`--ui-background${sufijo}`, variante.ui.background],
	[`--ui-surface${sufijo}`, variante.ui.surface],
	[`--ui-border${sufijo}`, variante.ui.border],
	[`--text-main${sufijo}`, variante.ui.text.main],
	[`--text-muted${sufijo}`, variante.ui.text.muted],
	[`--text-on-primary${sufijo}`, variante.ui.text["on-primary"]],
	[`--status-error${sufijo}`, variante.terminal?.ansi?.red],
	[`--status-success${sufijo}`, variante.terminal?.ansi?.green],
	[`--status-warning${sufijo}`, variante.terminal?.ansi?.yellow],
];

function aplicarEsquema(esquema: SchemeDocument) {
	const raiz = document.documentElement;

	for (const [variante, sufijo] of [
		[esquema.colors.light, ""],
		[esquema.colors.dark, "-dark"],
	] as const) {
		for (const [variable, color] of asignaciones(variante, sufijo)) {
			// Un esquema puede no traer la sección del terminal; en ese caso la
			// variable se deja como está en vez de escribirle «undefined», que
			// dejaría el color sin definir.
			if (color) raiz.style.setProperty(variable, color);
		}
	}
}

function aplicarTema(tema: string) {
	const raiz = document.documentElement;

	if (tema === "light") {
		raiz.classList.remove("dark");
		// El motor dibuja sus propios controles —barras de desplazamiento, el
		// cursor de texto, el anillo de foco— según esto, y en una pantalla
		// clara con `color-scheme: dark` salen oscuros sobre claro.
		raiz.style.setProperty("color-scheme", "light");
		return;
	}

	raiz.classList.add("dark");
	raiz.style.setProperty("color-scheme", "dark");
}

/** Se llama una vez, lo antes posible: son colores, y se ven. */
export async function loadAppearance(): Promise<void> {
	try {
		const apariencia = await invoke<Appearance>("get_appearance");

		aplicarTema(apariencia.theme);
		if (apariencia.scheme) aplicarEsquema(apariencia.scheme);
	} catch (reason) {
		// Sin colores configurados el inicio de sesión sigue siendo usable, así
		// que esto no interrumpe nada; queda en el log para poder verlo.
		console.warn("No se pudo leer la apariencia del inicio de sesión:", reason);
	}
}
