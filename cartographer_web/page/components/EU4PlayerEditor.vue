<script setup lang="ts">
import { ref } from "vue";
import type { EU4SaveGame } from "../types";

const emits = defineEmits<{
    confirm: [];
}>();
const model = defineModel<EU4SaveGame>({ required: true });

const add_name = ref("");
const add_tag = ref("");
function on_click_add() {
    const tag = add_tag.value.toUpperCase();
    if (!model.value.all_nations.has(tag)) {
        alert(`Unknown tag ${tag}`);
        return;
    }
    model.value.player_tags.set(tag, add_name.value);
    add_name.value = "";
    add_tag.value = "";
}
</script>
<template>
    <div>
        <div class="grid grid-cols-3">
            <div class="p-1">Player</div>
            <div class="p-1">Tag</div>
            <div></div>
            <template v-for="[tag, player] of model.player_tags" :key="player">
                <div class="p-1">
                    {{ player }}
                </div>
                <div class="p-1">
                    {{ tag }}
                </div>
                <button
                    @click="model.player_tags.delete(player)"
                    class="cursor-pointer"
                    title="Remove Player"
                >
                    X
                </button>
            </template>
            <input
                type="text"
                v-model="add_name"
                placeholder="Username"
                class="p-1"
            />
            <input
                type="text"
                v-model="add_tag"
                placeholder="Tag"
                class="p-1"
            />
            <button
                @click="on_click_add"
                class="cursor-pointer"
                title="Add Player"
            >
                +
            </button>
        </div>
        <button
            @click="emits('confirm')"
            class="border pt-0.5 pb-0.5 pl-1 pr-1"
        >
            Confirm
        </button>
    </div>
</template>
