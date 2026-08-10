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
  session_id: string | null;
}
