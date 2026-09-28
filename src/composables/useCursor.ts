import { ref } from 'vue'

interface RGB {
  r: number
  g: number
  b: number
}

const ARROW_POINTS: [number, number][] = [
  [0, 0],
  [0, 17.5],
  [4.4, 13.5],
  [7, 19.3],
  [9.5, 18.2],
  [6.9, 12.6],
  [11.5, 12.6]
]
const SCALE = 1.1
const OFFSET = 3.0
const OUTLINE_INFLATE = 3
const FILL_INFLATE = 0.9

function parseHex(hex: string): RGB | null {
  let v = hex.trim().replace(/^#/, '')
  if (v.length === 3) v = v.split('').map((c) => c + c).join('')
  if (v.length !== 6 || /[^0-9a-f]/i.test(v)) return null
  return {
    r: parseInt(v.slice(0, 2), 16),
    g: parseInt(v.slice(2, 4), 16),
    b: parseInt(v.slice(4, 6), 16)
  }
}

function rgbToHsl({ r, g, b }: RGB) {
  const rr = r / 255
  const gg = g / 255
  const bb = b / 255
  const max = Math.max(rr, gg, bb)
  const min = Math.min(rr, gg, bb)
  const l = (max + min) / 2
  let h = 0
  let s = 0
  if (max !== min) {
    const d = max - min
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
    if (max === rr) h = (gg - bb) / d + (gg < bb ? 6 : 0)
    else if (max === gg) h = (bb - rr) / d + 2
    else h = (rr - gg) / d + 4
    h *= 60
  }
  return { h, s: s * 100, l: l * 100 }
}

/**
 * Harmonisation du curseur avec la couleur d'accent (recherche UI) :
 * - remplissage = teinte analogue de l'accent (hue +15°, saturation relevée,
 *   luminosité contrôlée) : « se marie » avec l'accent sans être identique,
 *   donc visible par-dessus les boutons accentués ;
 * - contour neutre noir (thèmes clairs) ou blanc (thèmes sombres) : les deux
 *   couleurs à fort contraste garantissent la lisibilité sur tout fond ;
 * - accent achromatique (mono, graphite) → curseur blanc/noir classique.
 */
function deriveColors(accentHex: string, bgHex: string) {
  const accent = parseHex(accentHex) ?? { r: 170, g: 59, b: 255 }
  const bg = parseHex(bgHex) ?? { r: 255, g: 255, b: 255 }
  const { h, s } = rgbToHsl(accent)
  const dark = rgbToHsl(bg).l < 35

  if (s < 12) {
    return { fill: '#ffffff', outline: '#000000' }
  }

  const hue = (h + 15) % 360
  const sat = Math.min(90, Math.max(55, s + 10))
  const light = dark ? 50 : 40
  return {
    fill: `hsl(${Math.round(hue)}, ${Math.round(sat)}%, ${light}%)`,
    outline: dark ? '#ffffff' : '#000000'
  }
}

function renderCursor(fill: string, outline: string): string {
  const canvas = document.createElement('canvas')
  canvas.width = 32
  canvas.height = 32
  const ctx = canvas.getContext('2d')
  if (!ctx) return ''

  const trace = () => {
    ctx.beginPath()
    ARROW_POINTS.forEach(([x, y], i) => {
      const px = x * SCALE + OFFSET
      const py = y * SCALE + OFFSET
      if (i === 0) ctx.moveTo(px, py)
      else ctx.lineTo(px, py)
    })
    ctx.closePath()
  }

  ctx.lineJoin = 'round'
  ctx.lineCap = 'round'

  // contour : gonfle le polygone avec coins arrondis (rayon = 1.5 px)
  trace()
  ctx.strokeStyle = outline
  ctx.fillStyle = outline
  ctx.lineWidth = OUTLINE_INFLATE
  ctx.stroke()
  ctx.fill()

  // remplissage : laisse un anneau de contour d'environ 1 px
  trace()
  ctx.strokeStyle = fill
  ctx.fillStyle = fill
  ctx.lineWidth = FILL_INFLATE
  ctx.stroke()
  ctx.fill()

  return canvas.toDataURL('image/png')
}

let lastKey = ''

const STORAGE_KEY = 'agrobox_cursor'
export const customCursorEnabled = ref(localStorage.getItem(STORAGE_KEY) !== 'system')

function applyState() {
  lastKey = ''
  if (customCursorEnabled.value) {
    document.documentElement.setAttribute('data-cursor', 'custom')
    refreshCursor()
  } else {
    document.documentElement.removeAttribute('data-cursor')
    document.documentElement.style.removeProperty('--cursor')
  }
}

export function setCustomCursor(on: boolean) {
  customCursorEnabled.value = on
  localStorage.setItem(STORAGE_KEY, on ? 'custom' : 'system')
  applyState()
}

function refreshCursor() {
  if (!customCursorEnabled.value) return

  const styles = getComputedStyle(document.documentElement)
  const accent = styles.getPropertyValue('--accent').trim() || '#aa3bff'
  const bg = styles.getPropertyValue('--bg').trim() || '#ffffff'
  const { fill, outline } = deriveColors(accent, bg)

  const key = `${fill}|${outline}`
  if (key === lastKey) return
  lastKey = key

  const url = renderCursor(fill, outline)
  if (url) {
    document.documentElement.style.setProperty('--cursor', `url("${url}") 2 2`)
  }
}

export function initCursor() {
  applyState()

  let raf = 0
  const observer = new MutationObserver(() => {
    cancelAnimationFrame(raf)
    raf = requestAnimationFrame(refreshCursor)
  })
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['style', 'data-theme']
  })
}
