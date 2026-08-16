<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

// Временная реализация фейкового спрайта для разработки поведения окна питомца.
const sprite = ref<HTMLElement | null>(null);

// Подгоняет размер нативного окна под реальный размер спрайта на экране.
async function fitWindowToSprite() {
  const el = sprite.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  // Получаем инстанс окна tauri, и устанавливаем через его api размеры.
  // Тут происходит invoke события js->rust, происходит проверка capabilities, они есть,
  // и в итоге rust backend вызывает нативный macos resize из NSWindow.
  await getCurrentWindow().setSize(new LogicalSize(rect.width, rect.height));
}

onMounted(() => {
  fitWindowToSprite();
});
</script>

<template>
  <main>
    <!-- data-tauri-drag-region позволяет перетащить окно за спрайт. -->
    <div ref="sprite" class="sprite" data-tauri-drag-region />
  </main>
</template>

<style>
html,
body,
#app,
main {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  background: transparent;
}

.sprite {
  width: 128px;
  height: 128px;
  background-color: #ffb6a3;
  border-radius: 16px;
}
</style>
