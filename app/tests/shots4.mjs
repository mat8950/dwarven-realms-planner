// Headless check of DPS selection (basic attack + offhand toggles).
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
await page.waitForTimeout(1500)
await page.getByRole('button', { name: 'DPS', exact: true }).click()
await page.waitForTimeout(400)
const totals = async () => (await page.locator('.total .value').allTextContents()).join(' / ')
console.log('basic + offhands:', await totals(), '|', await page.locator('.opts').textContent())
await page.locator('.row.ab', { hasText: 'Lightning Plasma' }).click()
await page.waitForTimeout(200)
console.log('basic only:', await totals())
await page.locator('.row.atk', { hasText: 'Lance Magique' }).click()
await page.waitForTimeout(200)
console.log('Lance Magique only:', await totals())
await page.getByText('Afficher toutes les capacités').click()
await page.locator('.row.ab', { hasText: 'Chain Lightning' }).click()
await page.waitForTimeout(200)
console.log('Lance Magique + Chain Lightning:', await totals(), '|', await page.locator('.opts').textContent())
await page.screenshot({ path: `${out}/v4-dps.png` })
await browser.close()
