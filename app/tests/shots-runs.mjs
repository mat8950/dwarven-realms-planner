// Headless check of the Runs tab: node tests/shots-runs.mjs <outDir>  (needs `serve` + `vite` running)
import { chromium } from 'playwright-core'
import { homedir } from 'node:os'

const out = process.argv[2] ?? '.'
const exe = `${homedir()}/.cache/ms-playwright/chromium_headless_shell-1248/chrome-headless-shell-linux64/chrome-headless-shell`
const browser = await chromium.launch({ executablePath: exe, args: ['--no-sandbox'] })
const page = await browser.newPage({ viewport: { width: 1480, height: 960 } })
page.on('console', (m) => ['error', 'warning'].includes(m.type()) && console.log('console:', m.type(), m.text().slice(0, 300)))
page.on('pageerror', (e) => console.log('pageerror:', String(e.stack ?? e).slice(0, 600)))
await page.addInitScript(() => localStorage.setItem('dr-planner.lang', 'fr'))
await page.goto('http://localhost:5173/')
await page.waitForSelector('.row', { timeout: 30000 })
await page.getByRole('button', { name: 'Session', exact: true }).click()
await page.waitForSelector('.tile', { timeout: 60000 })
await page.screenshot({ path: `${out}/session-2.png` })
await page.getByRole('button', { name: 'Runs', exact: true }).click()
await page.waitForTimeout(1500); await page.screenshot({ path: `${out}/runs-0.png` }); await page.waitForSelector('.runs', { timeout: 10000 })
await page.waitForTimeout(500)
console.log('runs:', (await page.locator('.runs .trow:not(.head)').allTextContents()).map((s) => s.replace(/\s+/g, ' ').trim()).join(' | '))
console.log('rerolls placeholders:', await page.locator('.runs .trow:not(.head) input[type=number]:nth-of-type(3)').evaluateAll((els) => els.map((e) => e.getAttribute('placeholder'))))
await page.screenshot({ path: `${out}/runs-1.png` })
await page.locator('.runs .when').first().click()
await page.waitForTimeout(300)
await page.screenshot({ path: `${out}/runs-2.png` })
await browser.close()
