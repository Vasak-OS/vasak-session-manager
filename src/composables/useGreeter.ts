import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type {
  KeyboardLayout,
  LastLogin,
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

const selectedUser = ref<SystemUser | null>(null);
const selectedSession = ref<Session | null>(null);
/** Set when signing in as an account that is not in the list. */
const manualUsername = ref("");
const usingManualEntry = ref(false);

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

export function useGreeter() {
  async function load() {
    if (loaded.value) return;

    // Independent lookups: one of them failing must not blank the whole
    // screen, since the user can still sign in by typing their name.
    const [userList, sessionList, layout, last] = await Promise.all([
      invoke<SystemUser[]>("get_users").catch(() => [] as SystemUser[]),
      invoke<Session[]>("get_sessions").catch(() => [] as Session[]),
      invoke<KeyboardLayout>("get_keyboard_layout").catch(() => ({
        layouts: [],
        switchable: false,
      })),
      invoke<LastLogin>("get_last_login").catch(() => ({
        username: null,
        session_id: null,
      })),
    ]);

    users.value = userList;
    sessions.value = sessionList;
    keyboard.value = layout;

    selectedUser.value =
      userList.find((user) => user.name === last.username) ??
      userList[0] ??
      null;
    selectedSession.value =
      sessionList.find((session) => session.id === last.session_id) ??
      sessionList[0] ??
      null;

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
    selectedUser,
    selectedSession,
    manualUsername,
    usingManualEntry,
    username,
    load,
    selectUser,
    useManualEntry,
    rememberChoice,
  };
}
