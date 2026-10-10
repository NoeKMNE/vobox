// Génère updates/stable.json (manifeste Tauri updater) depuis les artefacts signés.
// Usage : node scripts/gen-updater-json.mjs --tag v0.2.0 --repo NoeKMNE/vobox --assets artifacts --out updates/stable.json
//
// Cherche récursivement les fichiers `*.sig`, en déduit la plateforme depuis le
// nom de l'archive associée et pointe vers l'asset de la GitHub Release.
import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import { basename, dirname, join } from 'node:path'

const args = Object.fromEntries(
  process.argv.slice(2).reduce((acc, cur, i, arr) => {
    if (cur.startsWith('--')) acc.push([cur.slice(2), arr[i + 1]])
    return acc
  }, [])
)

const tag = args.tag ?? ''
const repo = args.repo ?? ''
const assetsDir = args.assets ?? 'artifacts'
const out = args.out ?? 'updates/stable.json'
const version = tag.replace(/^v/, '').trim()

if (!version || !repo) {
  console.error('Usage : --tag vX.Y.Z --repo owner/name [--assets dir] [--out file]')
  process.exit(1)
}

const walk = (dir) => {
  let files = []
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry)
    if (statSync(full).isDirectory()) files = files.concat(walk(full))
    else files.push(full)
  }
  return files
}

// windows-x86_64 | windows-aarch64 | darwin-x86_64 | darwin-aarch64 | linux-x86_64 | linux-aarch64
const detectPlatform = (assetName) => {
  const n = assetName.toLowerCase()
  const arm = n.includes('aarch64') || n.includes('arm64')
  if (n.endsWith('.nsis.zip') || n.endsWith('.msi.zip')) {
    return arm ? 'windows-aarch64' : 'windows-x86_64'
  }
  if (n.endsWith('.app.tar.gz') || n.endsWith('.app.tar.gz.sig')) {
    if (arm || n.includes('aarch64')) return 'darwin-aarch64'
    return 'darwin-x86_64'
  }
  if (n.endsWith('.appimage.tar.gz')) {
    return arm ? 'linux-aarch64' : 'linux-x86_64'
  }
  return null
}

const sigs = walk(assetsDir).filter((f) => f.endsWith('.sig'))
const platforms = {}

for (const sigFile of sigs) {
  const assetFile = sigFile.slice(0, -'.sig'.length)
  const assetName = basename(assetFile)
  const platform = detectPlatform(assetName)
  if (!platform) {
    console.warn(`Plateforme indéterminée, ignoré : ${assetName}`)
    continue
  }
  if (platforms[platform]) {
    console.warn(`Doublon ${platform}, on garde le premier : ${assetName}`)
    continue
  }
  const signature = readFileSync(sigFile, 'utf8').trim()
  if (!signature) {
    console.warn(`Signature vide, ignoré : ${sigFile}`)
    continue
  }
  platforms[platform] = {
    signature,
    url: `https://github.com/${repo}/releases/download/${tag}/${assetName}`
  }
}

if (Object.keys(platforms).length === 0) {
  console.error(`Aucun artefact updater trouvé dans ${assetsDir}`)
  process.exit(1)
}

const manifest = {
  version,
  notes: `Vobox ${tag}`,
  pub_date: new Date().toISOString(),
  platforms
}

mkdirSync(dirname(out), { recursive: true })
writeFileSync(out, JSON.stringify(manifest, null, 2) + '\n')
console.log(`Manifeste écrit : ${out} (${Object.keys(platforms).join(', ')})`)
