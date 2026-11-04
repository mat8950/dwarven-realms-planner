// Checks the planner's translations: every t('key') used in src/ exists in the French
// reference dictionary, and every dictionary has the reference's keys.
// usage: node tests/i18n-check.mjs   (exit code 1 on a problem)
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const src = fileURLToPath(new URL('../src/', import.meta.url))
const keysOf = (file) => new Set([...readFileSync(file, 'utf8').matchAll(/^\s*'([\w.]+)':/gm)].map((m) => m[1]))

const ref = keysOf(join(src, 'lib/locales/fr.ts'))
let problems = 0

for (const f of readdirSync(join(src, 'lib/locales'))) {
  const keys = keysOf(join(src, 'lib/locales', f))
  const missing = [...ref].filter((k) => !keys.has(k))
  if (missing.length) {
    problems += missing.length
    console.log(`${f}: ${missing.length} missing key(s): ${missing.join(', ')}`)
  }
}

const walk = (d) => readdirSync(d).flatMap((n) => (statSync(join(d, n)).isDirectory() ? walk(join(d, n)) : [join(d, n)]))
const used = new Set()
for (const f of walk(src).filter((f) => /\.(svelte|ts)$/.test(f) && !f.includes('/locales/'))) {
  for (const m of readFileSync(f, 'utf8').matchAll(/\bt\(\s*'([\w.]+)'/g)) {
    used.add(m[1])
    if (!ref.has(m[1])) {
      problems++
      console.log(`${f.replace(src, 'src/')}: unknown key ${m[1]}`)
    }
  }
}
const unused = [...ref].filter((k) => !used.has(k) && !/^(rarity|slot|stance|race|skillset|setup\.step|setup\.store)\./.test(k))
if (unused.length) console.log(`unused keys (not an error): ${unused.join(', ')}`)
console.log(problems ? `${problems} problem(s)` : `ok: ${ref.size} keys, ${used.size} used directly`)
process.exit(problems ? 1 : 0)
