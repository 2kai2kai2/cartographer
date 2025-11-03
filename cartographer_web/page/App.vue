<script setup lang="ts">
/// <reference types="umami-browser" />
import { computed, ref } from "vue";
import FileUploadPage from "./components/FileUploadPage.vue";
import EU4PlayerEditor from "./components/EU4PlayerEditor.vue";
import {
    parse_save_file,
    render_stats_image_eu4,
    render_stats_image_stellaris,
} from "../pkg/cartographer_web";
import type { EU4SaveGame, StellarisSaveGame } from "./types";
import { OhVueIcon } from "oh-vue-icons";

const img_value = ref<string>("");
type SaveGameT = ["EU4", EU4SaveGame] | ["Stellaris", StellarisSaveGame];
const save_game = ref<SaveGameT | undefined>();
const stage = ref<
    "file_upload" | "parsing" | "player_edit" | "rendering" | "img_display"
>("file_upload");
const as_eu4_save = computed({
    get() {
        if (save_game.value?.[0] == "EU4") {
            return save_game.value[1];
        }
    },
    set(v: EU4SaveGame) {
        if (save_game.value?.[0] == "EU4") {
            save_game.value[1] = v;
        }
    },
});
const clicked_copy_img = ref(false);
async function on_click_copy_img() {
    const res = await fetch(img_value.value);
    await navigator.clipboard.write([
        new ClipboardItem({
            "image/png": res.blob(),
        }),
    ]);
    if (!clicked_copy_img.value) {
        // unless it's broken, we don't care about multiple clicks
        umami.track("clicked-copy-image");
    }
    clicked_copy_img.value = true;
}

async function do_rendering() {
    switch (save_game.value?.[0]) {
        case "EU4": {
            const img_b64 = await render_stats_image_eu4(save_game.value[1]);
            umami.track("render-completed");
            img_value.value = `data:image/png;base64,${img_b64}`;
            save_game.value = undefined; // free up memory
            stage.value = "img_display";
            break;
        }
        case "Stellaris":
            const img_b64 = await render_stats_image_stellaris(
                save_game.value[1]
            );
            umami.track("render-completed");
            img_value.value = `data:image/png;base64,${img_b64}`;
            save_game.value = undefined; // free up memory
            stage.value = "img_display";
            break;
        default:
            throw new Error("Invalid game. This is unlikely to happen.");
    }
}
async function on_file_picked(file: File) {
    console.log("Picked");
    stage.value = "parsing";
    const bytes = new Uint8Array(await file.arrayBuffer());
    try {
        const _save: SaveGameT = parse_save_file(bytes, file.name);
        save_game.value = _save;

        const player_count = _save[1].player_tags.size;
        umami.track("file-upload", {
            game: _save[0],
            mod: _save[1].game_mod,
            player_count,
            date: JSON.stringify(_save[1].date),
        });

        if (_save[0] == "EU4") {
            stage.value = "player_edit";
        } else {
            // TODO: do we need Stellaris player edit?
            // The game seems pretty reliable at remembering players, but might be useful
            stage.value = "rendering";
            do_rendering();
        }
    } catch (err) {
        console.error(err);
        stage.value = "file_upload";
        alert(`ERROR WHILE PARSING SAVE:\n${err}`);
    }
}
async function on_player_edit_confirm() {
    umami.track("player-edit-confirm");
    stage.value = "rendering";
    do_rendering();
}
</script>

<template>
    <header class="bg-blue-950 flex p-4">
        <h1 class="text-gray-100">The Cartographer</h1>
    </header>
    <main class="flex-grow m-2 flex flex-col items-center justify-center">
        <FileUploadPage
            @file_picked="on_file_picked"
            v-if="stage == 'file_upload'"
        />
        <template v-else-if="stage == 'parsing'">
            <OhVueIcon
                name="fa-spinner"
                animation="spin-pulse"
                scale="4"
                class="fill-blue-950"
            />
        </template>
        <template v-else-if="stage == 'player_edit'">
            <EU4PlayerEditor
                v-if="as_eu4_save"
                v-model="as_eu4_save"
                @confirm="on_player_edit_confirm"
            />
        </template>
        <template v-else-if="stage == 'rendering'">
            <OhVueIcon
                name="fa-spinner"
                animation="spin-pulse"
                scale="4"
                class="fill-blue-950"
            />
        </template>
        <template v-else-if="stage == 'img_display'">
            <div class="relative">
                <img :src="img_value" ref="img" />
                <div class="absolute right-1 top-1 flex gap-1">
                    <button
                        class="p-1 bg-gray-100 border-2 rounded-sm border-solid border-gray-500 hover:border-gray-600 flex cursor-pointer opacity-50 hover:opacity-100 transition-opacity"
                        title="Copy to Clipboard"
                        @click="on_click_copy_img"
                    >
                        <OhVueIcon
                            :name="
                                clicked_copy_img
                                    ? 'hi-clipboard-check'
                                    : 'hi-clipboard-copy'
                            "
                            class="stroke-gray-500 aspect-square w-auto"
                        />
                    </button>
                    <a
                        class="p-1 bg-gray-100 border-2 rounded-sm border-solid border-gray-500 hover:border-gray-600 flex cursor-pointer opacity-50 hover:opacity-100 transition-opacity"
                        title="Download"
                        :href="img_value"
                        :download="`cartographer_${new Date().toLocaleDateString()}`"
                    >
                        <OhVueIcon
                            name="fa-file-download"
                            class="fill-gray-500 aspect-square w-auto"
                        />
                    </a>
                </div>
            </div>
        </template>
    </main>
    <footer class="bg-blue-950 flex p-4">
        <a
            title="The Cartographer - GitHub Repository"
            href="https://github.com/2kai2kai2/cartographer"
        >
            <OhVueIcon name="fa-github" class="fill-gray-100" />
        </a>
    </footer>
</template>
