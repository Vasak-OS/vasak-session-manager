import { createApp } from "vue";
import { createPinia } from "pinia";
import I18n from "@vasakgroup/tauri-plugin-i18n";
import App from "./App.vue";
import LockView from "./LockView.vue";
import "./style.css";

// Two screens, one bundle: the greeter and the lock screen are the same
// interface over different moments — one before there is a session, one over a
// session that already exists. The window URL is what says which is being
// drawn.
const isLock = window.location.hash.startsWith("#/lock");

// The greeter runs before any user session/theme is available; default to the
// VasakOS dark scheme (typical for a login screen). The lock screen has a
// configuration to read, and the config store applies it over this.
document.documentElement.classList.add("dark");

const app = createApp(isLock ? LockView : App);
app.use(createPinia());

I18n.getInstance().load();

app.mount("#app");
