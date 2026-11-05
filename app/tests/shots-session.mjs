// Headless check of the Session view: node tests/shots-session.mjs <outDir>  (needs `serve` + `vite` running)
import { chromium } from 'playwright-core'
import { homedir } from 'node:os'

const out = process.argv[2] ?? '.'
const exe = `${homedir()}/.cache/ms-playwright/chromium_headless_shell-1248/chrome-headless-shell-linux64/chrome-headless-shell`
const browser = await chromium.launch({ executablePath: exe, args: ['--no-sandbox'] })
const page = await browser.newPage({ viewport: { width: 1480, height: 960 } })
page.on('console', (m) => m.type() === 'error' && console.log('console:', m.text()))
page.on('pageerror', (e) => console.log('pageerror:', e.message))
await page.addInitScript(() => localStorage.setItem('dr-planner.lang', 'fr'))
await page.goto('http://localhost:5173/')
await page.waitForSelector('.row', { timeout: 30000 })
await page.waitForTimeout(800)
await page.getByRole('button', { name: 'Session', exact: true }).click()
await page.waitForSelector('.tile', { timeout: 60000 })
await page.waitForTimeout(800)
console.log('tiles:', (await page.locator('.tile').allTextContents()).map((s) => s.replace(/\s+/g, ' ').trim()).join(' | '))
console.log('since:', (await page.locator('.pick select option').allTextContents()).join(' | '))
console.log('gains:', (await page.locator('.col .row').allTextContents()).slice(0, 12).map((s) => s.replace(/\s+/g, ' ').trim()).join(' | '))
console.log('steps:', (await page.locator('.step').allTextContents()).slice(0, 5).map((s) => s.replace(/\s+/g, ' ').trim()).join(' | '))
await page.screenshot({ path: `${out}/session-1.png` })
await browser.close()
