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

/**
 * El video de fondo, descrito antes de traerlo: la página decide con esto si
 * vale la pena pedir los bytes, sin haberlos pedido todavía.
 */
export interface BackgroundVideo {
  /** Sólo para los mensajes de error; nunca vuelve a Rust. */
  path: string;
  /** Con qué preguntarle a WebKit si tiene el decodificador. */
  extension: string;
  /** El tipo del `Blob` con el que se reproduce. */
  mime: string;
  bytes: number;
}

/** El fondo del inicio de sesión: la imagen siempre, el video cuando hay uno. */
export interface Background {
  /** `data:` URL, o null cuando no se encontró ninguna imagen usable. */
  image: string | null;
  video: BackgroundVideo | null;
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
