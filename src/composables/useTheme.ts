import { ref } from 'vue'

export type Theme = 'light' | 'dark' | 'mono' | 'tropical' | 'emerald'

const STORAGE_KEY = 'agrobox_theme'
const THEMES: Theme[] = ['light', 'dark', 'mono', 'tropical', 'emerald']

function getInitialTheme(): Theme {
  const stored = localStorage.getItem(STORAGE_KEY) as Theme | null
  if (stored && THEMES.includes(stored)) return stored
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
  return prefersDark ? 'dark' : 'light'
}

const currentTheme = ref<Theme>(getInitialTheme())

function applyTheme(theme: Theme) {
  document.documentElement.setAttribute('data-theme', theme)
}

export function initTheme() {
  applyTheme(currentTheme.value)
}

function setTheme(theme: Theme) {
  currentTheme.value = theme
  localStorage.setItem(STORAGE_KEY, theme)
  applyTheme(theme)
}

export function useTheme() {
  return { currentTheme, setTheme, initTheme, themes: THEMES }
}