import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type {
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
const background = ref<string | null>(null);
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
        invoke<string | null>("get_background").catch(() => null),
      ]);

    users.value = userList;
    sessions.value = sessionList;
    keyboard.value = keyboardLayout;
    rememberedSessions.value = last.sessions ?? {};
    layout.value = screens;
    background.value = wallpaper;

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
