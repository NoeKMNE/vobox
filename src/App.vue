<script setup lang="ts">
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { clearToken, getSession, getToken } from './composables/useAuth'

const router = useRouter()

onMounted(async () => {
  const token = getToken()

  if (!token) {
    router.push({ name: 'Login' })
    return
  }

  try {
    const user = await getSession(token)
    if (user.onboarding_done) {
      router.push({ name: 'Main' })
    } else {
      router.push({ name: 'Onboarding', params: { step: 1 } })
    }
  } catch {
    clearToken()
    router.push({ name: 'Login' })
  }
})
</script>

<template>
  <router-view v-slot="{ Component, route }">
    <transition name="page-root" mode="out-in">
      <component :is="Component" :key="route.matched[0]?.path" />
    </transition>
  </router-view>
</template>
