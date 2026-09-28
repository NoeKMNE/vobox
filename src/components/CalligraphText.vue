<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import * as React from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { Calligraph } from 'calligraph'
import type { CalligraphProps } from 'calligraph'

const props = withDefaults(defineProps<{
  text: string | number
  variant?: CalligraphProps['variant']
  animation?: CalligraphProps['animation']
  trend?: CalligraphProps['trend']
  stagger?: CalligraphProps['stagger']
  initial?: boolean
  autoSize?: boolean
  className?: string
}>(), {
  variant: 'text',
  animation: 'smooth',
  stagger: 0.02,
  initial: false,
  autoSize: true,
  className: ''
})

const host = ref<HTMLSpanElement | null>(null)
let root: Root | null = null

const renderCalligraph = () => {
  if (!root) return
  root.render(
    React.createElement(
      Calligraph,
      {
        variant: props.variant,
        animation: props.animation,
        trend: props.trend,
        stagger: props.stagger,
        initial: props.initial,
        autoSize: props.autoSize,
        className: props.className,
        as: 'span'
      },
      String(props.text)
    )
  )
}

onMounted(() => {
  if (host.value) {
    root = createRoot(host.value)
    renderCalligraph()
  }
})

watch(
  () => [props.text, props.variant, props.animation, props.trend, props.stagger, props.initial, props.autoSize, props.className],
  renderCalligraph
)

onBeforeUnmount(() => {
  root?.unmount()
  root = null
})
</script>

<template>
  <span ref="host" class="calligraph-host" :class="className"></span>
</template>

<style scoped>
.calligraph-host {
  display: inline;
}
</style>