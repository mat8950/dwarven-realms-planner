// Headless check of the key stats block while the analysis runs: node tests/shots-keys.mjs <outDir>  (needs `serve` + `vite`)
import { chromium } from 'playwright-core'
import { homedir } from 'node:os'

const out = process.argv[2] ?? '.'
const exe = `${homedir()}/.cache/ms-playwright/chromium_headless_shell-1248/chrome-headless-shell-linux64/chrome-headless-shell`
const browser = await chromium.launch({ executablePath: exe, args: ['--no-sandbox'] })
const page = await browser.newPage({ viewport: { width: 1480, height: 960 } })
page.on('pageerror', (e) => console.log('pageerror:', String(e.stack ?? e).slice(0, 300)))
await page.addInitScript(() => localStorage.setItem('dr-planner.lang', 'fr'))
await page.goto('http://localhost:5173/')
await page.waitForSelector('.key', { timeout: 30000 })
console.log('loading:', (await page.locator('.key').textContent())?.replace(/\s+/g, ' ').trim())
await page.locator('.key').screenshot({ path: `${out}/keys-loading.png` })
await page.waitForSelector('.key .krow:not(.ghost)', { timeout: 120000 })
console.log('loaded:', (await page.locator('.key').textContent())?.replace(/\s+/g, ' ').trim().slice(0, 200))
await page.locator('.key').screenshot({ path: `${out}/keys-loaded.png` })
await browser.close()
