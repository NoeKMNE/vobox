<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import QRCode from 'qrcode'

const props = withDefaults(defineProps<{ value: string; size?: number }>(), {
  size: 44
})

const url = ref('')

const render = async () => {
  try {
    // Rendu en PNG haute définition : la lib écrase le style des canvas,
    // donc on affiche une image dont la taille est forcée par notre CSS.
    url.value = await QRCode.toDataURL(props.value, {
      width: Math.max(props.size * 4, 176),
      margin: 1,
      errorCorrectionLevel: 'L',
      color: { dark: '#111827', light: '#ffffff' }
    })
  } catch {
    url.value = ''
  }
}

onMounted(render)
watch([() => props.value, () => props.size], render)
</script>

<template>
  <img
    v-if="url"
    :src="url"
    class="qr"
    alt="QR de traçabilité"
    :style="{ width: `${size}px`, height: `${size}px` }"
  />
</template>

<style scoped>
.qr {
  display: block;
  image-rendering: pixelated;
}
</style>
