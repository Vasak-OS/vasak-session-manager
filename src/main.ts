import { createApp } from "vue";
import I18n from "@vasakgroup/tauri-plugin-i18n";
import App from "./App.vue";
import "./style.css";

// The greeter runs before any user session/theme is available; default to the
// VasakOS dark scheme (typical for a login screen).
document.documentElement.classList.add("dark");

const app = createApp(App);

I18n.getInstance().load();

app.mount("#app");
