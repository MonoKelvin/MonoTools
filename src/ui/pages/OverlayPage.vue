<script setup lang="ts">
defineProps<{
  maxWidth?: string
}>()
</script>

<template>
  <div class="overlay-page">
    <div class="overlay-page__container">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.overlay-page {
  width: 100%;
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: flex-start;
  background: transparent;
  overflow: hidden;
  position: relative;
}

.overlay-page__container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: transparent;
  border-radius: 0;
  border-top: 1px solid rgba(255, 255, 255, 0.07);
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.55);
  overflow: hidden;
  animation: overlay-fade-in 220ms var(--ease-out);
}

.os-win10 .overlay-page__container {
  background: rgba(18, 18, 21, 0.85);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
}

@keyframes overlay-fade-in {
  0% {
    opacity: 0;
    transform: translateY(-8px) scale(0.985);
    filter: blur(6px);
  }
  100% {
    opacity: 1;
    transform: translateY(0) scale(1);
    filter: blur(0);
  }
}
</style>
