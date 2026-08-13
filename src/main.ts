import { mount } from "svelte";
import App from "./App.svelte";
import "./theme.css";
import "./app.css";
import "./settings.css";
import "./library.css";
import "./editor.css";

const target = document.getElementById("app");

if (!target) {
  throw new Error("Encore could not find its application mount point.");
}

const app = mount(App, { target });

export default app;
