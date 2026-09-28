<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import QRCode from 'qrcode'

const props = withDefaults(defineProps<{ value: string; size?: number }>(), {
  size: 110
})

const canvas = ref<HTMLCanvasElement | null>(null)

const render = async () => {
  const el = canvas.value
  if (!el) return
  try {
    await QRCode.toCanvas(el, props.value, {
      width: props.size,
      margin: 1,
      errorCorrectionLevel: 'L',
      color: { dark: '#111827', light: '#ffffff' }
    })
  } catch {
    // valeur non encodable : QR ignoré
  }
}

onMounted(render)
watch(() => props.value, render)
</script>

<template>
  <canvas
    ref="canvas"
    class="qr"
    :style="{ width: `${size}px`, height: `${size}px` }"
  />
</template>