export interface Accent {
  name: string
  hex: string
}

const STORAGE_KEY = 'Vobox_accent'

/** Graines de couleur Material (seed colors) : chaque graine engendre une palette tonale M3 */
export const ACCENTS: Accent[] = [
  { name: 'Cyan néon', hex: '#00e5ff' },
  { name: 'Magenta néon', hex: '#ff2ec4' },
  { name: 'Vert laser', hex: '#39ff88' },
  { name: 'Violet électrique', hex: '#b026ff' },
  { name: 'Ambre néon', hex: '#ffb300' },
  { name: 'Bleu électrique', hex: '#2979ff' },
  { name: 'Rouge laser', hex: '#ff3355' },
  { name: 'Graphite', hex: '#494057' }
]

interface Hsl {
  h: number
  s: number
}

function parseHex(hex: string): Hsl {
  let v = hex.trim().replace(/^#/, '')
  if (v.length === 3) v = v.split('').map((c) => c + c).join('')
  const r = parseInt(v.slice(0, 2), 16) / 255
  const g = parseInt(v.slice(2, 4), 16) / 255
  const b = parseInt(v.slice(4, 6), 16) / 255
  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  const l = (max + min) / 2
  const d = max - min
  if (d === 0) return { h: 260, s: 0 }
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
  let h = 0
  if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) * 60
  else if (max === g) h = ((b - r) / d + 2) * 60
  else h = ((r - g) / d + 4) * 60
  return { h, s: s * 100 }
}

const tone = (h: number, s: number, l: number) =>
  `hsl(${Math.round(h)} ${Math.round(Math.min(100, Math.max(0, s)))}% ${Math.round(l)}%)`

/** Palette tonale dérivée d'une graine, adaptée au thème clair ou sombre */
function buildTones(hex: string, dark: boolean) {
  const { h, s } = parseHex(hex)
  const chroma = Math.max(s, 20)
  const muted = chroma * 0.32
  const tertiaryHue = (h + 60) % 360

  if (dark) {
    return {
      primary: tone(h, chroma * 0.9, 78),
      onPrimary: tone(h, chroma * 0.6, 24),
      primaryContainer: tone(h, chroma * 0.8, 32),
      onPrimaryContainer: tone(h, chroma * 0.7, 90),
      secondary: tone(h, muted, 78),
      onSecondary: tone(h, muted, 22),
      secondaryContainer: tone(h, muted, 30),
      onSecondaryContainer: tone(h, muted, 88),
      tertiary: tone(tertiaryHue, chroma * 0.7, 80),
      onTertiaryContainer: tone(tertiaryHue, chroma * 0.6, 90)
    }
  }

  return {
    primary: tone(h, chroma * 0.85, 42),
    onPrimary: '#ffffff',
    primaryContainer: tone(h, chroma * 0.7, 90),
    onPrimaryContainer: tone(h, chroma * 0.75, 16),
    secondary: tone(h, muted, 42),
    onSecondary: '#ffffff',
    secondaryContainer: tone(h, muted, 90),
    onSecondaryContainer: tone(h, muted, 18),
    tertiary: tone(tertiaryHue, chroma * 0.6, 38),
    onTertiaryContainer: tone(tertiaryHue, chroma * 0.6, 16)
  }
}

function isDarkTheme() {
  const theme = document.documentElement.getAttribute('data-theme')
  return theme === 'dark' || theme === 'neon'
}

let lastSeed = ''

function applyAccent(hex: string) {
  const root = document.documentElement
  const tones = buildTones(hex, isDarkTheme())

  lastSeed = hex

  for (const [key, value] of Object.entries(tones)) {
    const cssVar = '--' + key.replace(/[A-Z]/g, (c) => '-' + c.toLowerCase())
    root.style.setProperty(cssVar, value)
  }

  root.style.setProperty('--accent', tones.primary)
  root.style.setProperty('--accent-bg', `color-mix(in srgb, ${tones.primary} 12%, transparent)`)
  root.style.setProperty('--accent-border', `color-mix(in srgb, ${tones.primary} 48%, transparent)`)
}

function clearAccent() {
  const root = document.documentElement
  lastSeed = ''
  for (const key of [
    'primary',
    'on-primary',
    'primary-container',
    'on-primary-container',
    'secondary',
    'on-secondary',
    'secondary-container',
    'on-secondary-container',
    'tertiary',
    'on-tertiary-container',
    'accent',
    'accent-bg',
    'accent-border'
  ]) {
    root.style.removeProperty(`--${key}`)
  }
}

let observer: MutationObserver | null = null

function watchTheme() {
  observer?.disconnect()
  observer = new MutationObserver(() => {
    if (lastSeed) applyAccent(lastSeed)
  })
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme']
  })
}

export function initAccent() {
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored) applyAccent(stored)
  watchTheme()
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
