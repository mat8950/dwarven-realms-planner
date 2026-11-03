// Headless UI check: node tests/shots.mjs <outDir>  (needs `serve` + `vite` running)
import { chromium } from 'playwright-core'
import { homedir } from 'node:os'

const out = process.argv[2] ?? '.'
const exe = `${homedir()}/.cache/ms-playwright/chromium_headless_shell-1248/chrome-headless-shell-linux64/chrome-headless-shell`
const browser = await chromium.launch({ executablePath: exe, args: ['--no-sandbox'] })
const page = await browser.newPage({ viewport: { width: 1480, height: 960 } })
page.on('console', (m) => m.type() === 'error' && console.log('console:', m.text()))
page.on('pageerror', (e) => console.log('pageerror:', e.message))
await page.goto('http://localhost:5173/')
await page.waitForSelector('.row', { timeout: 30000 })
await page.waitForTimeout(800)

// Open the helmet, bump its first affix by +50%, then open the mod picker.
await page.locator('.slot').first().click()
await page.waitForSelector('.affix')
await page.screenshot({ path: `${out}/ui-editor.png` })
const input = page.locator('.affix input').first()
const v = parseFloat(await input.inputValue())
await input.fill(String(v * 1.5))
await input.press('Enter')
await page.waitForTimeout(800)
await page.screenshot({ path: `${out}/ui-delta.png` })
await page.getByText('+ Ajouter un mod').click()
await page.waitForTimeout(300)
await page.screenshot({ path: `${out}/ui-mods.png` })
console.log('affix', await page.locator('.affix .aname').first().textContent(), v, '->', v * 1.5)
console.log('cr', await page.locator('.cr').textContent(), await page.locator('.cr-delta').textContent().catch(() => ''))
await browser.close()
