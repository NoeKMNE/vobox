import { ref } from 'vue'

export type Theme = 'light' | 'dark' | 'mono' | 'bleu' | 'neon'

const STORAGE_KEY = 'Vobox_theme'
const THEMES: Theme[] = ['light', 'dark', 'mono', 'bleu', 'neon']

function getInitialTheme(): Theme {
  const stored = localStorage.getItem(STORAGE_KEY) as Theme | null
  if (stored && THEMES.includes(stored)) return stored
  return 'neon'
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