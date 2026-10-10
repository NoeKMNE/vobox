<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Archive, BookOpenCheck, ChevronsLeft, Database, FileText, LayoutDashboard, LogOut, Settings, Sparkles, User } from '@lucide/vue'
import { clearToken, getSession, getToken, logout as apiLogout } from '../composables/useAuth'
import GuideTour from '../components/GuideTour.vue'
import UpdateDialog from '../components/UpdateDialog.vue'

const route = useRoute()
const router = useRouter()

const user = ref<{ name: string; email: string } | null>(null)

const sidebarCollapsed = ref(localStorage.getItem('Vobox_sidebar_collapsed') === 'true')

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
  localStorage.setItem('Vobox_sidebar_collapsed', String(sidebarCollapsed.value))
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
        <span class="brand-name" data-label>Vobox</span>
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
          :class="{ active: isActive('Assistant') }"
          @click.prevent="goSection('Assistant')"
        >
          <span class="nav-icon"><Sparkles :size="18" /></span>
          <span class="nav-label" data-label>Assistant IA</span>
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
          :class="{ active: isActive('Archive') || route.name === 'ProducerHistory' }"
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
        <a
          href="#"
          class="nav-link"
          :class="{ active: isActive('Guide') }"
          @click.prevent="goSection('Guide')"
        >
          <span class="nav-icon"><BookOpenCheck :size="18" /></span>
          <span class="nav-label" data-label>Guide</span>
        </a>
      </nav>

      <div class="sidebar-footer">
        <div class="sidebar-user">
          <span class="user-avatar"><User :size="18" /></span>
          <div class="user-meta">
            <span class="user-name-text" data-label>{{ user?.name || 'Utilisateur' }}</span>
            <span class="user-plan" data-label>{{ user?.email || 'Plan Vobox' }}</span>
          </div>
          <button class="logout-btn" title="Déconnexion" aria-label="Déconnexion" @click="logout">
            <LogOut :size="16" />
          </button>
        </div>
      </div>
    </aside>

    <main class="shell-main">
      <router-view v-slot="{ Component }">
        <component :is="Component" :key="route.fullPath" />
      </router-view>
    </main>
    <GuideTour />
    <UpdateDialog />
  </div>
</template>

<style scoped>
.app-shell {
  min-height: 100svh;
  display: flex;
  gap: 12px;
  padding: 12px;
  box-sizing: border-box;
  background: var(--surface);
}

/* ---------- Sidebar : rail de navigation M3 ---------- */
.sidebar {
  position: sticky;
  top: 12px;
  align-self: flex-start;
  height: calc(100svh - 24px);
  width: 260px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px 12px;
  box-sizing: border-box;
  background: var(--surface-container);
  border: none;
  border-radius: var(--shape-xl);
  box-shadow: var(--elev-1);
  transition: width 0.25s cubic-bezier(0.2, 0, 0, 1);
}

.sidebar.collapsed {
  width: 80px;
  padding: 16px 12px;
}

/* ---------- Brand ---------- */
.sidebar-brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 2px 4px;
}

.brand-toggle {
  width: 40px;
  height: 40px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  background: var(--primary);
  color: var(--on-primary);
  border: none;
  border-radius: var(--shape-md);
  cursor: pointer;
  box-shadow: none;
  transition: transform var(--motion);
}

.brand-toggle svg {
  transition: transform 0.25s ease;
}

.brand-toggle:hover {
  box-shadow: var(--elev-1);
}

.brand-toggle:active {
  transform: scale(0.94);
}

.sidebar.collapsed .brand-toggle svg {
  transform: rotate(180deg);
}

.brand-name {
  font-size: 20px;
  font-weight: 700;
  letter-spacing: 0.1px;
  color: var(--on-surface);
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
  gap: 12px;
  height: 56px;
  padding: 0 16px;
  border-radius: var(--shape-full);
  color: var(--on-surface-variant);
  text-decoration: none;
  font-size: 14px;
  font-weight: 500;
  letter-spacing: 0.1px;
  position: relative;
  transition:
    background var(--motion),
    color var(--motion);
}

.nav-link:hover {
  background: color-mix(in srgb, var(--on-surface) 8%, transparent);
  color: var(--on-surface);
}

.nav-link.active {
  background: var(--secondary-container);
  color: var(--on-secondary-container);
  font-weight: 600;
}

.nav-icon {
  width: 24px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  color: var(--on-surface-variant);
}

.nav-link.active .nav-icon {
  color: var(--on-secondary-container);
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
  gap: 12px;
  padding: 10px 12px;
  background: var(--surface-container-high);
  border: none;
  border-radius: var(--shape-full);
}

.user-avatar {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  border-radius: var(--shape-full);
  background: var(--primary-container);
  color: var(--on-primary-container);
  font-weight: 600;
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
  border-radius: var(--shape-full);
  color: var(--text);
  cursor: pointer;
  transition: color var(--motion), background var(--motion);
}

.logout-btn:hover {
  color: var(--error);
  background: var(--error-container);
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
  .sidebar .user-meta {
    display: none;
  }
  /* Le bouton déconnexion reste accessible : icône seule */
  .sidebar .logout-btn {
    display: grid;
  }
  .sidebar .sidebar-user {
    flex-wrap: wrap;
    row-gap: 6px;
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
