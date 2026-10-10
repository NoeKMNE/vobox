// Harmonise la version depuis le tag git (ex : v0.2.0 -> 0.2.0)
// vers src-tauri/tauri.conf.json + src-tauri/Cargo.toml.
// Usage CI : node scripts/sync-version.mjs "v0.2.0"
import { readFileSync, writeFileSync } from 'node:fs'

const tag = process.argv[2] ?? ''
const version = tag.replace(/^v/, '').trim()

if (!/^\d+\.\d+\.\d+/.test(version)) {
  console.error(`Tag invalide (attendu vX.Y.Z) : ${tag}`)
  process.exit(1)
}

// tauri.conf.json
const confPath = new URL('../src-tauri/tauri.conf.json', import.meta.url)
const conf = JSON.parse(readFileSync(confPath, 'utf8'))
conf.version = version
writeFileSync(confPath, JSON.stringify(conf, null, 2) + '\n')

// Cargo.toml (ligne `version = "..."` du [package])
const cargoPath = new URL('../src-tauri/Cargo.toml', import.meta.url)
const cargo = readFileSync(cargoPath, 'utf8').replace(
  /(\[package\][^\[]*?^version\s*=\s*")[^"]+(")/m,
  `$1${version}$2`
)
writeFileSync(cargoPath, cargo)

console.log(`Version synchronisée : ${version}`)
