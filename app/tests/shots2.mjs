// Headless check of forge hearts, capacities tab, objective and filters.
import { chromium } from 'playwright-core'
import { homedir } from 'node:os'

const out = process.argv[2] ?? '.'
const exe = `${homedir()}/.cache/ms-playwright/chromium_headless_shell-1248/chrome-headless-shell-linux64/chrome-headless-shell`
const browser = await chromium.launch({ executablePath: exe, args: ['--no-sandbox'] })
const page = await browser.newPage({ viewport: { width: 1480, height: 960 } })
page.on('pageerror', (e) => console.log('pageerror:', e.message))
page.on('console', (m) => m.type() === 'error' && console.log('console:', m.text()))
await page.goto('http://localhost:5173/')
await page.waitForSelector('.row', { timeout: 30000 })
await page.waitForTimeout(2500) // first analysis
const cr = async () => (await page.locator('.cr').textContent()).trim()
console.log('CR start', await cr())

// Helmet: 3 hearts instead of 1.
await page.locator('.slot').first().click()
await page.waitForSelector('.hearts')
await page.locator('.heart').nth(2).click()
await page.waitForTimeout(2000)
console.log('CR helmet 3 hearts', await cr(), await page.locator('.cr-delta').textContent().catch(() => ''))
await page.screenshot({ path: `${out}/v2-hearts.png` })

// Capacities tab, Lightning Plasma as objective, then the affix picker sorted for it.
await page.getByRole('button', { name: 'Capacités' }).click()
await page.waitForTimeout(300)
await page.screenshot({ path: `${out}/v2-caps.png` })
await page.locator('.cap', { hasText: 'Lightning Plasma' }).first().click()
await page.waitForTimeout(2500)
await page.getByText('+ Ajouter un affixe').click()
await page.waitForTimeout(400)
await page.screenshot({ path: `${out}/v2-picker.png` })
const top = await page.locator('.choices .choice').evaluateAll((els) => els.slice(0, 6).map((e) => e.textContent.replace(/\s+/g, ' ').trim()))
console.log('top affixes for LP:', top)

// Hide useless stats, back to stats tab.
await page.getByText('Masquer les stats inutiles').click()
await page.getByRole('button', { name: 'Stats', exact: true }).click()
await page.waitForTimeout(400)
await page.screenshot({ path: `${out}/v2-filter.png` })

// Forge mode: no hearts anywhere.
await page.locator('select').nth(4).selectOption('none').catch(async () => {
  const sels = page.locator('select'); const n = await sels.count()
  for (let i = 0; i < n; i++) if ((await sels.nth(i).innerHTML()).includes('Sans cœurs')) await sels.nth(i).selectOption('none')
})
await page.waitForTimeout(1500)
console.log('CR no hearts', await cr(), await page.locator('.cr-delta').textContent().catch(() => ''))

// Weapon swap.
await page.locator('.weapon .slot').first().click()
await page.getByText("⇄ Changer d'arme").click()
await page.locator('.picker input').fill('Graveforged')
await page.locator('.choices .choice').first().click()
await page.waitForTimeout(1500)
console.log('after swap', (await page.locator('.editor .name').textContent()).trim(), (await page.locator('.lvl').textContent()).trim(), await cr())
await page.screenshot({ path: `${out}/v2-swap.png` })
await browser.close()
