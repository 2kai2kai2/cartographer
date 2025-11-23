<script setup lang="ts">
import { ref } from "vue";

const emits = defineEmits<{
    file_picked: [File];
}>();

function on_drop(ev: DragEvent) {
    const blob = ev.dataTransfer?.files.item(0);
    if (blob != null) {
        emits("file_picked", blob);
    }
}

let dragover = ref(false);
function on_dragover(ev: DragEvent) {
    if (ev.dataTransfer?.types.includes("Files")) {
        dragover.value = true;
    }
}
function on_dragleave() {
    dragover.value = false;
}

function on_change(ev: Event) {
    const target = ev.target! as HTMLInputElement;
    const item = target.files!.item(0);
    if (item != null) {
        emits("file_picked", item);
    }
}
</script>

<template>
    <label
        :class="[
            'bg-gray-200',
            'flex flex-col items-center justify-center',
            'border-gray-400 border-solid border rounded-2xl p-16 text-gray-500 transition-colors cursor-pointer',
            dragover
                ? 'hover:bg-blue-200 hover:border-blue-400 hover:text-blue-400 hover:fill-blue-400'
                : 'hover:bg-gray-300 hover:border-gray-600 hover:text-gray-600 hover:fill-gray-400',
        ]"
        @drop.stop.prevent="on_drop"
        @dragleave="on_dragleave"
        @dragover.prevent="on_dragover"
        ref="file-upload-drop"
    >
        <input
            type="file"
            class="hidden"
            accept=".eu4,.eu5,.sav"
            @change="on_change"
        />
        <div class="m-2">
            Upload Save File
            <v-icon name="fa-upload" class="ml-2" />
        </div>
        <i class="text-sm">Any non-ironman EU4, EU5, or Stellaris save file</i>
    </label>
</template>
