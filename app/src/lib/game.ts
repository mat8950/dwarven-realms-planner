// Visual rules taken from the game's widgets (names are in lib/locales).
import { linearToCss } from './format'

/** UI_ItemSlot: rarity -> background texture and frame tint (linear colours from the blueprint). */
export const RARITY = [
  { bg: 'EasySurvivalRPG/Textures/UI/T_UI_Button_Frame_002.webp', frame: linearToCss(0.098958, 0.098958, 0.098958) },
  { bg: 'EasySurvivalRPG/Textures/UI/magic-bg.webp', frame: linearToCss(0.0, 0.195801, 1.0) },
  { bg: 'EasySurvivalRPG/Textures/UI/unique-bg.webp', frame: linearToCss(0.981097, 1.0, 0.5625) },
  { bg: 'EasySurvivalRPG/Textures/UI/epic-bg.webp', frame: linearToCss(0.659387, 0.059963, 0.885417) },
  { bg: 'EasySurvivalRPG/Textures/UI/flawless-bg.webp', frame: linearToCss(1.0, 0.114967, 0.015308) },
]
export const PET = { bg: 'EasySurvivalRPG/Textures/UI/pet-bg.webp', frame: linearToCss(0.0, 0.744792, 0.085906) }

/** Paperdoll layout of the in-game character screen (E_EquipmentSlot values). */
export const LEFT_SLOTS = [0, 1, 3, 15, 2, 4]
export const RIGHT_SLOTS = [8, 14, 9, 6, 7, 5]
export const TOP_SLOT = 10
export const WEAPON_CHILDREN = [11, 12, 13]
export const PET_SLOT = 14

export const MOD_PREFIX = 'EasyRPG.Items.Modifiers.'
const ABILITY_MOD = /^EasyRPG\.Attributes\.Abilities\.[^.]+\.Modifier\./
/** Equipment mods / monograms, and offhand ability modifiers (AdditionalBeam...). */
export const isMod = (tag: string) => tag.startsWith(MOD_PREFIX) || ABILITY_MOD.test(tag)
export const abilityOfMod = (tag: string) => (ABILITY_MOD.test(tag) ? tag.split('.Modifier.')[0] : null)

export const asset = (p: string | null | undefined) => (p ? '/' + p : '')
