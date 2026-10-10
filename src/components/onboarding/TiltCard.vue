<script setup lang="ts">
withDefaults(
  defineProps<{
    maxTilt?: number
    glare?: boolean
  }>(),
  { maxTilt: 0, glare: false }
)
</script>

<template>
  <div class="mdc-elevated">
    <div class="mdc-elevated-card">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.mdc-elevated {
  display: block;
}

.mdc-elevated-card {
  position: relative;
  height: 100%;
  border-radius: var(--shape-lg);
  background: var(--surface-container-low);
  box-shadow: var(--elev-1);
  transition: box-shadow var(--motion);
}

.mdc-elevated-card:hover {
  box-shadow: var(--elev-2);
}
</style>
