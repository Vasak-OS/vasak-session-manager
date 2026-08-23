import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type {
  Background,
  BackgroundVideo,
  KeyboardLayout,
  LastLogin,
  Screen,
  ScreenLayout,
  Session,
  SystemUser,
} from "@/types/greeter";

/**
 * Everything the login screen needs, loaded once and shared.
 *
 * Kept outside the composable so the user list and the session list survive
 * component remounts — the greeter is a single screen and reloading them on
 * every toggle would re-read every account and desktop file for nothing.
 */
const users = ref<SystemUser[]>([]);
const sessions = ref<Session[]>([]);
const keyboard = ref<KeyboardLayout>({ layouts: [], switchable: false });
/** `data:` URL de la imagen de fondo: el respaldo de todo lo demás. */
const background = ref<string | null>(null);
/** El video configurado, descrito por Rust pero todavía sin traer. */
const backgroundVideo = ref<BackgroundVideo | null>(null);
/** `blob:` del video ya en memoria, que es lo único que se puede reproducir. */
const backgroundVideoUrl = ref<string | null>(null);
const layout = ref<ScreenLayout>({ width: 0, height: 0, screens: [] });
/** Index of the screen the login box is drawn on. */
const activeScreen = ref(0);

const selectedUser = ref<SystemUser | null>(null);
const selectedSession = ref<Session | null>(null);
/** Set when signing in as an account that is not in the list. */
const manualUsername = ref("");
const usingManualEntry = ref(false);

/** The session each account used last time, as read from disk. */
const rememberedSessions = ref<Record<string, string>>({});

const loaded = ref(false);

/**
 * Fondos en movimiento.
 *
 * Un `<video src>` apuntando al protocolo interno de Tauri no funciona, y no por
 * los codecs: el elemento multimedia de WebKit no se sirve del cargador de
 * recursos de la página sino de GStreamer, que no sabe leer de un esquema
 * propio. Termina en error 4 (SRC_NOT_SUPPORTED) y encima reintenta, y cada
 * reintento entrega el archivo entero hasta agotar la memoria. Con `file://`
 * pasa lo mismo, porque la página no es de ese origen.
 *
 * Lo que sí funciona —medido, y es lo que hace el escritorio— es traer los bytes
 * y reproducirlos desde memoria: acá los pide un comando de Rust, que es el que
 * puede leer el disco, y salen por el canal binario del IPC sin pasar por
 * base64. El costo es tener el archivo en RAM, de ahí el límite de tamaño.
 *
 * Nada de esto es obligatorio para que se pueda iniciar sesión: cada vez que
 * algo falla queda la imagen, que ya está dibujada.
 */
const MAX_VIDEO_BYTES = 64 * 1024 * 1024;

/**
 * Qué se le pregunta a WebKit para saber si puede con el archivo.
 *
 * Preguntar `video/mp4` a secas no sirve: contesta «maybe» aun cuando no hay
 * ningún decodificador instalado. Con el codec en la pregunta contesta vacío, y
 * ahí sí se puede decidir sin intentar. Un mp4 puede traer H.264 o AV1, así que
 * alcanza con que alguno de los dos sea reproducible.
 */
const CODEC_PROBES: Record<string, string[]> = {
  mp4: ['video/mp4; codecs="avc1.42E01E"', 'video/mp4; codecs="av01.0.04M.08"'],
  webm: ['video/webm; codecs="vp9"', 'video/webm; codecs="vp8"'],
  ogv: ['video/ogg; codecs="theora"'],
};

function canDecode(extension: string): boolean {
  const probe = document.createElement("video");
  const types = CODEC_PROBES[extension] ?? [`video/${extension}`];
  return types.some((type) => probe.canPlayType(type) !== "");
}

/** Suelta el video: la imagen de abajo vuelve a ser el fondo. */
function releaseBackgroundVideo() {
  if (backgroundVideoUrl.value) {
    URL.revokeObjectURL(backgroundVideoUrl.value);
    backgroundVideoUrl.value = null;
  }
}

/**
 * Los bytes vuelven por el canal binario del IPC, que los entrega como
 * `ArrayBuffer` —una vista, sin copiar los megas—. Las otras dos formas están
 * por si el IPC cae al camino de `postMessage`: son una copia, pero son también
 * la diferencia entre un fondo y una pantalla negra.
 */
function asBytes(raw: unknown): Uint8Array<ArrayBuffer> | null {
  if (raw instanceof ArrayBuffer) return new Uint8Array(raw);
  if (raw instanceof Uint8Array) return new Uint8Array(raw);
  if (Array.isArray(raw)) return new Uint8Array(raw);
  return null;
}

async function startBackgroundVideo() {
  releaseBackgroundVideo();

  const video = backgroundVideo.value;
  if (!video) return;

  if (!canDecode(video.extension)) {
    console.error(
      `El fondo ${video.path} no se puede reproducir: falta el decodificador ` +
        `para ${video.extension}. Se muestra la imagen de fondo. En VasakOS ` +
        "lo instala gst-libav.",
    );
    return;
  }

  if (video.bytes > MAX_VIDEO_BYTES) {
    console.error(
      `El fondo ${video.path} pesa ${Math.round(video.bytes / 1024 / 1024)} MB ` +
        `y el límite es ${MAX_VIDEO_BYTES / 1024 / 1024} MB: se reproduce desde ` +
        "memoria, y esta pantalla es lo primero que arranca en la máquina.",
    );
    return;
  }

  try {
    const bytes = asBytes(await invoke("read_background_video"));
    if (!bytes || bytes.byteLength === 0) throw new Error("no llegaron bytes");

    backgroundVideoUrl.value = URL.createObjectURL(
      new Blob([bytes], { type: video.mime }),
    );
  } catch (reason) {
    console.error(`No se pudo leer el fondo ${video.path}: ${reason}`);
    releaseBackgroundVideo();
  }
}

/** The name to authenticate with, whichever way it was chosen. */
const username = computed(() =>
  usingManualEntry.value
    ? manualUsername.value.trim()
    : (selectedUser.value?.name ?? ""),
);

export function displayName(user: SystemUser): string {
  return user.real_name || user.name;
}

/**
 * Restores the session this account signed in with last time.
 *
 * Only ever *changes* the selection: an account with nothing remembered — or
 * whose desktop is no longer installed — keeps whatever is already chosen,
 * rather than being bounced back to the first session in the list.
 */
function recallSession(name: string) {
  const remembered = rememberedSessions.value[name];
  if (!remembered) return;

  const session = sessions.value.find((entry) => entry.id === remembered);
  if (session) selectedSession.value = session;
}

watch(username, (name) => {
  if (name) recallSession(name);
});

export function useGreeter() {
  async function load() {
    if (loaded.value) return;

    // Independent lookups: one of them failing must not blank the whole
    // screen, since the user can still sign in by typing their name.
    const [userList, sessionList, keyboardLayout, last, screens, wallpaper] =
      await Promise.all([
        invoke<SystemUser[]>("get_users").catch(() => [] as SystemUser[]),
        invoke<Session[]>("get_sessions").catch(() => [] as Session[]),
        invoke<KeyboardLayout>("get_keyboard_layout").catch(() => ({
          layouts: [],
          switchable: false,
        })),
        invoke<LastLogin>("get_last_login").catch(() => ({
          username: null,
          sessions: {},
        })),
        invoke<ScreenLayout>("get_screens").catch(() => ({
          width: 0,
          height: 0,
          screens: [] as Screen[],
        })),
        invoke<Background>("get_background").catch(
          () => ({ image: null, video: null }) as Background,
        ),
      ]);

    users.value = userList;
    sessions.value = sessionList;
    keyboard.value = keyboardLayout;
    rememberedSessions.value = last.sessions ?? {};
    layout.value = screens;
    background.value = wallpaper.image;
    backgroundVideo.value = wallpaper.video;

    activeScreen.value =
      screens.screens.find((screen) => screen.primary)?.index ?? 0;

    selectedUser.value =
      userList.find((user) => user.name === last.username) ??
      userList[0] ??
      null;
    selectedSession.value = sessionList[0] ?? null;
    if (selectedUser.value) recallSession(selectedUser.value.name);

    // Nobody to pick from: go straight to typing a name instead of showing an
    // empty list with no way forward.
    usingManualEntry.value = userList.length === 0;
    loaded.value = true;

    // Sin esperarlo: el cuadro de inicio de sesión ya está dibujado sobre la
    // imagen, y el video puede pesar decenas de megas. Que se pueda escribir la
    // contraseña no depende de esto.
    void startBackgroundVideo();
  }

  function selectUser(user: SystemUser) {
    selectedUser.value = user;
    usingManualEntry.value = false;
  }

  function useManualEntry() {
    usingManualEntry.value = true;
    selectedUser.value = null;
  }

  /**
   * Follows the pointer across the monitors.
   *
   * The coordinates are the ones the page is laid out in, and the surface
   * covers the whole output layout, so a pointer position is already a
   * position within the layout — no monitor lookup on the Rust side is needed,
   * and there is no Wayland call that would give one to a client anyway.
   */
  function pointerMoved(x: number, y: number) {
    const screen = layout.value.screens.find(
      (candidate) =>
        x >= candidate.x &&
        x < candidate.x + candidate.width &&
        y >= candidate.y &&
        y < candidate.y + candidate.height,
    );

    // A pointer in a gap between two monitors of different heights is not on
    // any of them; leaving the login box where it is beats moving it home.
    if (screen) activeScreen.value = screen.index;
  }

  function rememberChoice() {
    if (!username.value || !selectedSession.value) return;
    invoke("set_last_login", {
      username: username.value,
      sessionId: selectedSession.value.id,
    }).catch(() => {
      /* Remembering is a convenience; never surface it as a login error. */
    });
  }

  return {
    users,
    sessions,
    keyboard,
    background,
    backgroundVideoUrl,
    releaseBackgroundVideo,
    layout,
    activeScreen,
    selectedUser,
    selectedSession,
    manualUsername,
    usingManualEntry,
    username,
    load,
    selectUser,
    useManualEntry,
    pointerMoved,
    rememberChoice,
  };
}
