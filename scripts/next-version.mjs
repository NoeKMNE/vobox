// Calcule le prochain tag de release (bump patch) depuis le dernier tag `v*`.
// Sans tag existant : part de la version de src-tauri/tauri.conf.json.
// Usage : node scripts/next-version.mjs   -> affiche `vX.Y.Z` sur stdout
import { execSync } from 'node:child_process'
import { readFileSync } from 'node:fs'

const bump = (version) => {
  const m = version.trim().replace(/^v/, '').match(/^(\d+)\.(\d+)\.(\d+)/)
  if (!m) {
    console.error(`Version illisible : ${version}`)
    process.exit(1)
  }
  return `v${m[1]}.${m[2]}.${Number(m[3]) + 1}`
}

let base = ''
try {
  base = execSync('git describe --tags --abbrev=0 --match "v*"', {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore']
  }).trim()
} catch {
  const conf = JSON.parse(
    readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8')
  )
  base = `v${conf.version}`
}

console.log(bump(base))
