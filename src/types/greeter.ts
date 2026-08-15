export interface SystemUser {
  name: string;
  /** From the GECOS field. Empty when the account has no real name set. */
  real_name: string;
  uid: number;
  gid: number;
  home: string;
  shell: string;
  /** `data:` URL, or null when the account has no picture. */
  avatar: string | null;
}

export interface Session {
  id: string;
  name: string;
  comment: string;
  exec: string;
  path: string;
  session_type: string;
  desktop_names: string[];
}

export interface KeyboardLayout {
  layouts: string[];
  switchable: boolean;
}

export interface LastLogin {
  username: string | null;
  /** Session chosen by each account, keyed by user name. */
  sessions: Record<string, string>;
}

/** One physical monitor, in CSS pixels relative to the greeter surface. */
export interface Screen {
  index: number;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  /** Where the login box goes until the pointer says otherwise. */
  primary: boolean;
}

export interface ScreenLayout {
  width: number;
  height: number;
  screens: Screen[];
}
