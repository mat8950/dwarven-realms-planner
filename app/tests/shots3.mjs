// Headless check: DPS tab, debug panel, physical/elemental labels. URL from argv[3].
import { chromium } from 'playwright-core'
import { homedir } from 'node:os'

const out = process.argv[2] ?? '.'
const url = process.argv[3] ?? 'http://localhost:5173/'
const exe = `${homedir()}/.cache/ms-playwright/chromium_headless_shell-1248/chrome-headless-shell-linux64/chrome-headless-shell`
const browser = await chromium.launch({ executablePath: exe, args: ['--no-sandbox'] })
const page = await browser.newPage({ viewport: { width: 1480, height: 960 } })
page.on('pageerror', (e) => console.log('pageerror:', e.message))
await page.goto(url)
await page.waitForSelector('.row', { timeout: 30000 })
await page.waitForTimeout(2500)
const labels = await page.locator('.rows .row .lbl').allTextContents()
console.log('labels:', labels.filter((l) => /gâts|lémentaire/.test(l)).join(' | '))
await page.getByRole('button', { name: 'DPS', exact: true }).click()
await page.waitForTimeout(400)
await page.screenshot({ path: `${out}/v3-dps.png` })
console.log('totals:', (await page.locator('.total .value').allTextContents()).join(' / '))
await page.keyboard.press('Control+Shift+D')
await page.waitForTimeout(800)
await page.screenshot({ path: `${out}/v3-debug.png` })
await browser.close()
