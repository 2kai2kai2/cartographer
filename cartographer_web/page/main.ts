import { createApp } from "vue";
import App from "./App.vue";
import "./style.css";

import { OhVueIcon, addIcons } from "oh-vue-icons";
import {
    FaGithub,
    FaDiscord,
    FaPatreon,
    FaPlus,
    FaUpload,
    FaSpinner,
    FaFileDownload,
    HiClipboardCopy,
    HiClipboardCheck,
    MdPersonremoveRound,
} from "oh-vue-icons/icons";

import init_wasm from "../pkg/cartographer_web";
init_wasm();

addIcons(
    FaGithub,
    FaDiscord,
    FaPlus,
    FaPatreon,
    FaUpload,
    FaSpinner,
    FaFileDownload,
    HiClipboardCopy,
    HiClipboardCheck,
    MdPersonremoveRound
);

const app = createApp(App);
app.component("v-icon", OhVueIcon);
app.mount("#app");
