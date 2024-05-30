import "./style.scss";
import { createApp } from "vue";
import ElementPlus from "element-plus";
import "element-plus/dist/index.css";
import "element-plus/theme-chalk/dark/css-vars.css";
import App from "./App.vue";

const app = createApp(App);

document.documentElement.classList.add(
  window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light",
);

window
  .matchMedia("(prefers-color-scheme: dark)")
  .addEventListener("change", (event) => {
    const newColorScheme = event.matches ? "dark" : "light";
    document.documentElement.classList.remove("dark", "light");
    document.documentElement.classList.add(newColorScheme);
  });

app.use(ElementPlus);
app.mount("#app");
