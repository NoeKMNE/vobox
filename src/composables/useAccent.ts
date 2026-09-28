export interface Accent {
  name: string
  hex: string
}

const STORAGE_KEY = 'agrobox_accent'

export const ACCENTS: Accent[] = [
  { name: 'Violet', hex: '#aa3bff' },
  { name: 'Vert prairie', hex: '#2f9e44' },
  { name: 'Émeraude', hex: '#12b886' },
  { name: 'Océan', hex: '#1c7ed6' },
  { name: 'Ambre', hex: '#f08c00' },
  { name: 'Corail', hex: '#e8590c' },
  { name: 'Framboise', hex: '#e64980' },
  { name: 'Graphite', hex: '#495057' }
]

const withAlpha = (hex: string, alpha: string) => `${hex}${alpha}`

function applyAccent(hex: string) {
  const root = document.documentElement
  root.style.setProperty('--accent', hex)
  root.style.setProperty('--accent-bg', withAlpha(hex, '1f'))
  root.style.setProperty('--accent-border', withAlpha(hex, '80'))
}

function clearAccent() {
  const root = document.documentElement
  root.style.removeProperty('--accent')
  root.style.removeProperty('--accent-bg')
  root.style.removeProperty('--accent-border')
}

export function initAccent() {
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored) applyAccent(stored)
}

export function setAccent(hex: string) {
  localStorage.setItem(STORAGE_KEY, hex)
  applyAccent(hex)
}

export function resetAccent() {
  localStorage.removeItem(STORAGE_KEY)
  clearAccent()
}

export function currentAccentHex(): string {
  return localStorage.getItem(STORAGE_KEY) ?? ACCENTS[0].hex
}