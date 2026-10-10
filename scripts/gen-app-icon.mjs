import zlib from 'node:zlib'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const ROOT = path.join(__dirname, '..')
const OUT = path.join(ROOT, 'src', 'assets', 'app-icon.png')

const N = 1024

const clamp = (v, a, b) => Math.min(b, Math.max(a, v))
const lerp = (a, b, t) => a + (b - a) * t

// --- encodeur PNG (RGBA, filtre "Up") ---
const CRC_TABLE = (() => {
  const t = new Uint32Array(256)
  for (let n = 0; n < 256; n++) {
    let c = n
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
    t[n] = c >>> 0
  }
  return t
})()

function crc32(buf) {
  let c = 0xffffffff
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8)
  return (c ^ 0xffffffff) >>> 0
}

function chunk(type, data) {
  const len = Buffer.alloc(4)
  len.writeUInt32BE(data.length)
  const body = Buffer.concat([Buffer.from(type, 'latin1'), data])
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE(crc32(body))
  return Buffer.concat([len, body, crc])
}

function encodePng(width, height, rgba) {
  const stride = width * 4
  const raw = Buffer.alloc((stride + 1) * height)
  for (let y = 0; y < height; y++) {
    raw[y * (stride + 1)] = 0 // pas de filtre
    const prev = y > 0 ? (y - 1) * (stride + 1) + 1 : -1
    const cur = y * (stride + 1) + 1
    for (let i = 0; i < stride; i++) {
      const v = rgba[y * stride + i]
      raw[cur + i] = prev < 0 ? v : (v - rgba[(y - 1) * stride + i]) & 0xff
    }
  }
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(width, 0)
  ihdr.writeUInt32BE(height, 4)
  ihdr[8] = 8 // profondeur 8 bits
  ihdr[9] = 6 // RGBA
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', zlib.deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0))
  ])
}

// --- primitives géométriques (coordonnées normalisées 0..1, y vers le bas) ---
const P = {
  stem: { ax: 0.5, ay: 0.7, bx: 0.5, by: 0.52, r: 0.034 },
  branchL: { ax: 0.5, ay: 0.54, bx: 0.355, by: 0.36, r: 0.027 },
  branchR: { ax: 0.5, ay: 0.54, bx: 0.645, by: 0.36, r: 0.027 },
  leafL: { cx: 0.32, cy: 0.295, rx: 0.105, ry: 0.052, ang: (-140 * Math.PI) / 180 },
  leafR: { cx: 0.68, cy: 0.295, rx: 0.105, ry: 0.052, ang: (-40 * Math.PI) / 180 }
}

function distSegment(px, py, a, b) {
  const abx = b.ax - a.ax
  const aby = b.ay - a.ay
  const t = clamp(((px - a.ax) * abx + (py - a.ay) * aby) / (abx * abx + aby * aby), 0, 1)
  const dx = px - (a.ax + t * abx)
  const dy = py - (a.ay + t * aby)
  return Math.hypot(dx, dy)
}

function distEllipse(px, py, e) {
  const dx = px - e.cx
  const dy = py - e.cy
  const c = Math.cos(e.ang)
  const s = Math.sin(e.ang)
  const lx = c * dx - s * dy
  const ly = s * dx + c * dy
  return Math.hypot(lx / e.rx, ly / e.ry) - 1
}

function glyphCoverage(x, y) {
  let cov = 0
  const add = (d) => (cov = Math.max(cov, clamp(0.5 - d * N, 0, 1)))
  add(distSegment(x, y, P.stem, P.stem) - P.stem.r)
  add(distSegment(x, y, P.branchL, P.branchL) - P.branchL.r)
  add(distSegment(x, y, P.branchR, P.branchR) - P.branchR.r)
  add(distEllipse(x, y, P.leafL))
  add(distEllipse(x, y, P.leafR))
  return cov
}

// --- rendu ---
const M = 44 / N // marge
const RR = 220 / N // rayon des coins
const GRAD = { top: [177, 92, 255], bottom: [114, 40, 214] }

function roundRectDist(x, y) {
  const hw = 0.5 - M
  const dx = Math.abs(x - 0.5) - hw
  const dy = Math.abs(y - 0.5) - hw
  const ax = Math.max(dx, 0)
  const ay = Math.max(dy, 0)
  const rx = Math.max(dx + RR, 0)
  const ry = Math.max(dy + RR, 0)
  return Math.hypot(ax, ay) + Math.min(Math.max(Math.min(rx, ry), 0), RR) - RR
}

const raw = new Uint8Array(N * N * 4)
for (let y = 0; y < N; y++) {
  for (let x = 0; x < N; x++) {
    const nx = x / N
    const ny = y / N
    const d = roundRectDist(nx, ny) * N
    const bgCov = clamp(0.5 - d, 0, 1)
    if (bgCov <= 0) continue

    const t = clamp((ny - M) / (1 - 2 * M), 0, 1)
    let cr = lerp(GRAD.top[0], GRAD.bottom[0], t)
    let cg = lerp(GRAD.top[1], GRAD.bottom[1], t)
    let cb = lerp(GRAD.top[2], GRAD.bottom[2], t)

    // reflet doux en haut à gauche
    const sp = clamp(1 - Math.hypot(nx - 0.3, ny - 0.2) / 0.6, 0, 1)
    const hi = Math.pow(sp, 3) * 0.12
    cr += (255 - cr) * hi
    cg += (255 - cg) * hi
    cb += (255 - cb) * hi

    // pousse blanche
    const w = glyphCoverage(nx, ny)
    cr = lerp(cr, 255, w)
    cg = lerp(cg, 255, w)
    cb = lerp(cb, 255, w)

    const i = (y * N + x) * 4
    raw[i] = Math.round(cr)
    raw[i + 1] = Math.round(cg)
    raw[i + 2] = Math.round(cb)
    raw[i + 3] = Math.round(bgCov * 255)
  }
}

fs.mkdirSync(path.dirname(OUT), { recursive: true })
fs.writeFileSync(OUT, encodePng(N, N, raw))
console.log('written:', OUT, fs.statSync(OUT).size, 'bytes')