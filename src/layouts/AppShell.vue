<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Archive, ChevronsLeft, Database, FileText, LayoutDashboard, LogOut, Settings, User } from '@lucide/vue'
import CalligraphText from '../components/CalligraphText.vue'
import { clearToken, getSession, getToken, logout as apiLogout } from '../composables/useAuth'

const route = useRoute()
const router = useRouter()

const user = ref<{ name: string; email: string } | null>(null)

const sidebarCollapsed = ref(localStorage.getItem('agrobox_sidebar_collapsed') === 'true')

const loadUser = async () => {
  const token = getToken()
  if (!token) return
  try {
    const sessionUser = await getSession(token)
    user.value = { name: sessionUser.name, email: sessionUser.email }
  } catch {
    // session invalide : la déconnexion nettoiera l'état
  }
}

onMounted(loadUser)
watch(() => route.path, loadUser)

const toggleSidebar = () => {
  sidebarCollapsed.value = !sidebarCollapsed.value
  localStorage.setItem('agrobox_sidebar_collapsed', String(sidebarCollapsed.value))
}

const goSection = (name: string) => {
  router.push({ name })
}

const isActive = (name: string) => route.name === name

const logout = async () => {
  const token = getToken()
  if (token) {
    try {
      await apiLogout(token)
    } catch {
      // la session locale est nettoyée quoi qu'il arrive
    }
  }
  clearToken()
  router.push({ name: 'Login' })
}
</script>

<template>
  <div class="app-shell">
    <aside
      class="sidebar"
      :class="{ collapsed: sidebarCollapsed }"
      :aria-expanded="!sidebarCollapsed"
    >
      <div class="sidebar-brand">
        <button
          class="brand-toggle"
          :aria-expanded="!sidebarCollapsed"
          :aria-label="sidebarCollapsed ? 'Étendre le menu' : 'Réduire le menu'"
          :title="sidebarCollapsed ? 'Étendre le menu' : 'Réduire le menu'"
          @click="toggleSidebar"
        >
          <ChevronsLeft :size="18" />
        </button>
        <span class="brand-name" data-label><CalligraphText text="AgroBox" :initial="true" /></span>
      </div>

      <nav class="sidebar-nav">
        <a
          href="#"
          class="nav-link"
          :class="{ active: isActive('Main') }"
          @click.prevent="goSection('Main')"
        >
          <span class="nav-icon"><LayoutDashboard :size="18" /></span>
          <span class="nav-label" data-label>Accueil</span>
        </a>
        <a
          href="#"
          class="nav-link"
          :class="{ active: isActive('Settings') }"
          @click.prevent="goSection('Settings')"
        >
          <span class="nav-icon"><Settings :size="18" /></span>
          <span class="nav-label" data-label>Paramètres</span>
        </a>
        <a
          href="#"
          class="nav-link"
          :class="{ active: isActive('Sheet') }"
          @click.prevent="goSection('Sheet')"
        >
          <span class="nav-icon"><FileText :size="18" /></span>
          <span class="nav-label" data-label>Fiche</span>
        </a>
        <a
          href="#"
          class="nav-link"
          :class="{ active: isActive('Archive') }"
          @click.prevent="goSection('Archive')"
        >
          <span class="nav-icon"><Archive :size="18" /></span>
          <span class="nav-label" data-label>Archives</span>
        </a>
        <a
          href="#"
          class="nav-link"
          :class="{ active: isActive('Data') || route.name === 'Exploitation' }"
          @click.prevent="goSection('Data')"
        >
          <span class="nav-icon"><Database :size="18" /></span>
          <span class="nav-label" data-label>Données</span>
        </a>
      </nav>

      <div class="sidebar-footer">
        <div class="sidebar-user">
          <span class="user-avatar"><User :size="18" /></span>
          <div class="user-meta">
            <span class="user-name-text" data-label>{{ user?.name || 'Utilisateur' }}</span>
            <span class="user-plan" data-label>{{ user?.email || 'Plan AgroBox' }}</span>
          </div>
          <button class="logout-btn" title="Déconnexion" @click="logout">
            <LogOut :size="16" />
          </button>
        </div>
      </div>
    </aside>

    <main class="shell-main">
      <router-view v-slot="{ Component }">
        <transition name="page" mode="out-in">
          <component :is="Component" :key="route.fullPath" />
        </transition>
      </router-view>
    </main>
  </div>
</template>

<style scoped>
.app-shell {
  min-height: 100svh;
  display: flex;
  gap: 18px;
  padding: 16px;
  box-sizing: border-box;
  background: var(--code-bg);
}

/* ---------- Sidebar : carte flottante 3D ---------- */
.sidebar {
  position: sticky;
  top: 16px;
  align-self: flex-start;
  height: calc(100svh - 32px);
  width: 260px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 16px 14px;
  box-sizing: border-box;
  background: linear-gradient(180deg, var(--bg) 0%, var(--social-bg) 100%);
  border: 1px solid var(--border);
  border-radius: 24px;
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.08),
    0 1px 2px rgba(0, 0, 0, 0.06),
    0 12px 32px -8px rgba(0, 0, 0, 0.18),
    0 32px 64px -24px rgba(0, 0, 0, 0.25);
  transition: width 0.25s ease, transform 0.25s ease, box-shadow 0.25s ease;
}

.sidebar:hover {
  transform: translateY(-3px);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.09),
    0 2px 4px rgba(0, 0, 0, 0.06),
    0 18px 44px -8px rgba(0, 0, 0, 0.22),
    0 44px 80px -24px rgba(0, 0, 0, 0.3);
}

.sidebar::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.04);
}

.sidebar.collapsed {
  width: 72px;
  padding: 16px 10px;
}

/* ---------- Brand ---------- */
.sidebar-brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 2px 4px;
}

.brand-toggle {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  background: linear-gradient(135deg, var(--accent) 0%, color-mix(in srgb, var(--accent) 70%, #000 30%) 100%);
  color: #fff;
  border: none;
  border-radius: 12px;
  cursor: pointer;
  box-shadow: 0 4px 12px -2px color-mix(in srgb, var(--accent) 55%, transparent);
  transition: transform 0.2s ease, box-shadow 0.2s ease, filter 0.2s ease;
}

.brand-toggle svg {
  transition: transform 0.25s ease;
}

.brand-toggle:hover {
  transform: translateY(-1px) scale(1.05);
  filter: brightness(1.06);
  box-shadow: 0 6px 16px -2px color-mix(in srgb, var(--accent) 65%, transparent);
}

.brand-toggle:active {
  transform: scale(0.96);
}

.sidebar.collapsed .brand-toggle svg {
  transform: rotate(180deg);
}

.brand-name {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-h);
  white-space: nowrap;
  transition: opacity 0.2s ease;
}

/* ---------- Nav ---------- */
.sidebar-nav {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  overflow-y: auto;
  scrollbar-width: none;
  padding-right: 2px;
  margin-right: -2px;
}

.sidebar-nav::-webkit-scrollbar {
  display: none;
}

.nav-link {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 10px;
  border-radius: 12px;
  color: var(--text);
  text-decoration: none;
  font-size: 14px;
  font-weight: 500;
  position: relative;
  transition: background 0.2s ease, color 0.2s ease, box-shadow 0.2s ease;
}

.nav-link::before {
  content: '';
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%) scaleY(0);
  width: 3px;
  height: 60%;
  border-radius: 3px;
  background: var(--accent);
  transition: transform 0.2s ease;
}

.nav-link:hover {
  background: var(--accent-bg);
  color: var(--text-h);
}

.nav-link.active {
  background: var(--accent-bg);
  color: var(--text-h);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.04),
    0 2px 8px rgba(0, 0, 0, 0.05);
}

.nav-link.active::before {
  transform: translateY(-50%) scaleY(1);
}

.nav-icon {
  width: 24px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  color: var(--accent);
}

.nav-link.active .nav-icon {
  color: var(--accent);
}

.nav-label {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition: opacity 0.2s ease;
}

/* ---------- Footer : user ---------- */
.sidebar-footer {
  margin-top: auto;
  padding-top: 12px;
  border-top: 1px solid var(--border);
}

.sidebar-user {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 16px;
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.03);
}

.user-avatar {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  border-radius: 50%;
  background: linear-gradient(135deg, var(--accent) 0%, color-mix(in srgb, var(--accent) 70%, #000 30%) 100%);
  color: #fff;
  box-shadow: 0 4px 12px -2px color-mix(in srgb, var(--accent) 50%, transparent);
}

.user-meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.user-name-text {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-h);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.user-plan {
  font-size: 12px;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.logout-btn {
  width: 30px;
  height: 30px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  background: none;
  border: none;
  border-radius: 9px;
  color: var(--text);
  cursor: pointer;
  transition: color 0.2s ease, background 0.2s ease;
}

.logout-btn:hover {
  color: #dc2626;
  background: rgba(220, 38, 38, 0.1);
}

/* ---------- Collapsed overrides ---------- */
.sidebar.collapsed .sidebar-brand,
.sidebar.collapsed .sidebar-user {
  align-items: center;
}

.sidebar.collapsed .sidebar-brand {
  justify-content: center;
}

.sidebar.collapsed [data-label] {
  opacity: 0;
  pointer-events: none;
  max-width: 0;
  margin: 0;
}

.sidebar.collapsed .nav-link {
  justify-content: center;
  padding: 9px 0;
}

.sidebar.collapsed .nav-icon {
  width: auto;
}

.sidebar.collapsed .sidebar-user {
  justify-content: center;
  padding: 6px;
}

.sidebar.collapsed .sidebar-footer {
  border-top: none;
  padding-top: 0;
}

.sidebar.collapsed .brand-name,
.sidebar.collapsed .user-meta,
.sidebar.collapsed .logout-btn {
  display: none;
}

/* ---------- Main content ---------- */
.shell-main {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* ---------- Page transitions ---------- */
.page-enter-active,
.page-leave-active {
  transition: opacity 0.22s ease, transform 0.22s ease;
}

.page-enter-from {
  opacity: 0;
  transform: translateY(10px);
}

.page-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}

@media (prefers-reduced-motion: reduce) {
  .page-enter-active,
  .page-leave-active {
    transition: none;
  }
}

/* ---------- Responsive ---------- */
@media (max-width: 768px) {
  .sidebar:not(.collapsed) {
    width: 240px;
  }
}

@media (max-width: 560px) {
  .app-shell {
    padding: 10px;
    gap: 12px;
  }
  .sidebar {
    top: 10px;
    height: calc(100svh - 20px);
  }

  /* Forcer la version icônes seules pour libérer l'espace */
  .sidebar {
    width: 72px;
    padding: 16px 10px;
  }
  .sidebar .sidebar-brand,
  .sidebar .sidebar-user {
    align-items: center;
  }
  .sidebar .sidebar-brand {
    justify-content: center;
  }
  .sidebar [data-label] {
    opacity: 0;
    pointer-events: none;
    max-width: 0;
    margin: 0;
  }
  .sidebar .nav-link {
    justify-content: center;
    padding: 9px 0;
  }
  .sidebar .nav-icon {
    width: auto;
  }
  .sidebar .sidebar-user {
    justify-content: center;
    padding: 6px;
  }
  .sidebar .sidebar-footer {
    border-top: none;
    padding-top: 0;
  }
  .sidebar .brand-name,
  .sidebar .user-meta,
  .sidebar .logout-btn {
    display: none;
  }
  .sidebar .brand-toggle svg {
    transform: rotate(180deg);
  }
}

@media (max-width: 480px) {
  .app-shell {
    padding: 8px;
  }
  .sidebar {
    width: 64px;
    padding: 14px 8px;
  }
}
</style>