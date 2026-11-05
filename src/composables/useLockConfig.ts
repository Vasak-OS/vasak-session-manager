import { useConfigStore } from "@vasakgroup/plugin-config-manager";
import { computed } from "vue";

/**
 * Lo que la sección `lock` de `vasak.conf` dice sobre el aspecto de la pantalla
 * de bloqueo: el reloj, el fondo y qué elementos aparecen.
 *
 * Todo arranca encendido y con el aspecto de hoy: la sección no existe hasta que
 * alguien cambia algo en Configuración, y mientras no exista —o mientras la
 * `loadConfig()` de `LockView` todavía no llegó— la pantalla tiene que verse
 * exactamente como siempre. Por eso cada clave se lee tolerante (`!== false`,
 * tipo comprobado): el archivo se edita a mano y un valor raro no puede romper el
 * dibujo. El bloqueo nunca oculta el campo de contraseña ni el avatar; esto sólo
 * toca el reloj, el fondo, la tarjeta de medios y los avisos.
 *
 * El fondo propio lo resuelve el lado Rust (`lock_background` lee la misma
 * sección); acá se decide sólo el velo de atenuación, que es presentación.
 */
export type LockBackgroundSource = "desktop" | "image" | "color";

export function useLockConfig() {
  const configStore = useConfigStore();
  const section = computed(() => (configStore as any).config?.lock ?? {});
  const clock = computed(() => section.value.clock ?? {});
  const background = computed(() => section.value.background ?? {});

  const source = computed<LockBackgroundSource>(() => {
    const value = background.value.source;
    return value === "image" || value === "color" ? value : "desktop";
  });

  return {
    /** 12 o 24 h forzadas; sin clave, lo decide el idioma (`undefined`). */
    hour12: computed<boolean | undefined>(() =>
      typeof clock.value.hour12 === "boolean" ? clock.value.hour12 : undefined,
    ),
    /** La fecha debajo de la hora; visible salvo que `lock.clock.date` sea `false`. */
    showDate: computed(() => clock.value.date !== false),
    /** El tamaño del reloj; `lg` salvo que se pida `md`. */
    clockSize: computed<"md" | "lg">(() => (clock.value.size === "md" ? "md" : "lg")),
    /** De dónde sale el fondo: el del escritorio (como hoy), una imagen propia o el color del esquema. */
    backgroundSource: source,
    /**
     * Si el fondo se atenúa. El del escritorio se atenúa siempre, como hoy; la
     * imagen propia sólo si `lock.background.dim` no está en `false`. El color
     * liso no tiene imagen que atenuar.
     */
    dimBackground: computed(() =>
      source.value === "image" ? background.value.dim !== false : true,
    ),
    /** La tarjeta de medios; visible salvo que `lock.media` sea `false`. */
    showMedia: computed(() => section.value.media !== false),
    /** Los avisos sin leer; visibles salvo que `lock.notifications` sea `false`. */
    showNotifications: computed(() => section.value.notifications !== false),
  };
}
