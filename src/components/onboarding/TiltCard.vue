<script setup lang="ts">
import { ref } from 'vue'

const props = withDefaults(
  defineProps<{
    maxTilt?: number
    glare?: boolean
  }>(),
  { maxTilt: 10, glare: true }
)

const el = ref<HTMLElement | null>(null)
const transform = ref('perspective(1100px) rotateX(0deg) rotateY(0deg)')
const gx = ref('50%')
const gy = ref('50%')
const tilted = ref(false)

function onMove(e: MouseEvent) {
  const card = el.value
  if (!card) return
  const rect = card.getBoundingClientRect()
  const px = (e.clientX - rect.left) / rect.width
  const py = (e.clientY - rect.top) / rect.height
  const rx = ((0.5 - py) * props.maxTilt).toFixed(2)
  const ry = ((px - 0.5) * props.maxTilt).toFixed(2)
  transform.value = `perspective(1100px) rotateX(${rx}deg) rotateY(${ry}deg)`
  gx.value = `${(px * 100).toFixed(1)}%`
  gy.value = `${(py * 100).toFixed(1)}%`
  tilted.value = true
}

function onLeave() {
  transform.value = 'perspective(1100px) rotateX(0deg) rotateY(0deg)'
  tilted.value = false
}
</script>

<template>
  <div class="tilt">
    <div
      ref="el"
      class="tilt-card"
      :class="{ tilted }"
      :style="{ transform, '--gx': gx, '--gy': gy }"
      @mousemove="onMove"
      @mouseleave="onLeave"
    >
      <div v-if="props.glare" class="tilt-glare"></div>
      <slot />
    </div>
  </div>
</template>

<style scoped>
.tilt {
  perspective: 1100px;
}

.tilt-card {
  position: relative;
  transition: transform 0.15s ease-out;
  will-change: transform;
}

.tilt-glare {
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.2s ease;
  background: radial-gradient(circle at var(--gx) var(--gy), rgba(255, 255, 255, 0.16), transparent 55%);
}

.tilt-card.tilted .tilt-glare {
  opacity: 1;
}

@media (prefers-reduced-motion: reduce) {
  .tilt-card {
    transition: none;
  }
}
</style>