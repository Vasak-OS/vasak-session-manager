import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./style.css";

// The greeter runs before any user session/theme is available; default to the
// VasakOS dark scheme (typical for a login screen).
document.documentElement.classList.add("dark");

const app = createApp(App);
const pinia = createPinia();

app.use(pinia);

app.mount("#app");
