-- One-shot data dump for the Dwarven Realms build planner.
-- Loads every DataTable (so their row structs are in memory), then writes Mappings.usmap.
local Tables = {
    "/Game/BetaAcess/DT_Experiments.DT_Experiments",
    "/Game/Blueprints/Necro/AbilitySystem/DT_MinionSkills.DT_MinionSkills",
    "/Game/EasySurvivalRPG/Blueprints/Abilities/SkillAbilities/DT_OffhandCategoryGameplayTags.DT_OffhandCategoryGameplayTags",
    "/Game/EasySurvivalRPG/Blueprints/Abilities/SkillAbilities/Localized/DT_PlayerAbilities.DT_PlayerAbilities",
    "/Game/EasySurvivalRPG/Blueprints/Abilities/SkillAbilities/Localized/OffhandTags/DT_OffhandAffinities.DT_OffhandAffinities",
    "/Game/EasySurvivalRPG/Blueprints/Characters/Base/Races/DT_PlayerRaces.DT_PlayerRaces",
    "/Game/EasySurvivalRPG/Blueprints/Characters/Enemies/DT_BossVoiceLines_V2.DT_BossVoiceLines_V2",
    "/Game/EasySurvivalRPG/Blueprints/Characters/Npcs/Fellowship/DT_FellowshipEmblems.DT_FellowshipEmblems",
    "/Game/EasySurvivalRPG/Blueprints/Characters/Npcs/HomeFortFortuneTrader/DT_ExpeditionCheatTags.DT_ExpeditionCheatTags",
    "/Game/EasySurvivalRPG/Blueprints/Characters/Npcs/HomeFortFortuneTrader/DT_Expeditions.DT_Expeditions",
    "/Game/EasySurvivalRPG/Blueprints/Characters/Npcs/HomeFortFortuneTrader/DT_Ubers.DT_Ubers",
    "/Game/EasySurvivalRPG/Blueprints/Components/HomeFortStations/Localized/DT_HomeFort_StationsUpgrades.DT_HomeFort_StationsUpgrades",
    "/Game/EasySurvivalRPG/Blueprints/Components/SpawnedContainers/Localized/DT_ContainerSpawners.DT_ContainerSpawners",
    "/Game/EasySurvivalRPG/Blueprints/Components/StanceLogic/Necro/DT_MinionGroups.DT_MinionGroups",
    "/Game/EasySurvivalRPG/Blueprints/Components/StanceLogic/Necro/DT_MinionInfo.DT_MinionInfo",
    "/Game/EasySurvivalRPG/Blueprints/Components/StanceLogic/Necro/DT_MinionStatusEffects.DT_MinionStatusEffects",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/DT_Abilities.DT_Abilities",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/DT_Blueprints.DT_Blueprints",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/DT_BuildingLists.DT_BuildingLists",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/DT_BuildingObjects.DT_BuildingObjects",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/DT_Config_Util_Items.DT_Config_Util_Items",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/DT_CraftingLists.DT_CraftingLists",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/DT_Items.DT_Items",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/DT_Items_Abilities.DT_Items_Abilities",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/DT_LootLists.DT_LootLists",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/DT_Relationships.DT_Relationships",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/DT_Skills.DT_Skills",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/DT_Sounds.DT_Sounds",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/DT_TradeLists.DT_TradeLists",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/DT_UsableItems.DT_UsableItems",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GameplayTags/DT_GamePlayTags_Game.DT_GamePlayTags_Game",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GameplayTags/DT_GameplayTags_Attributes.DT_GameplayTags_Attributes",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GameplayTags/DT_GameplayTags_Attributes_AbilitySystem.DT_GameplayTags_Attributes_AbilitySystem",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GameplayTags/DT_GameplayTags_Attributes_DamageSystem.DT_GameplayTags_Attributes_DamageSystem",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GameplayTags/DT_GameplayTags_Attributes_Skills.DT_GameplayTags_Attributes_Skills",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GameplayTags/DT_GameplayTags_Attributes_StatusEffects.DT_GameplayTags_Attributes_StatusEffects",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GameplayTags/DT_GameplayTags_InterestPoints.DT_GameplayTags_InterestPoints",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GameplayTags/DT_GameplayTags_Items.DT_GameplayTags_Items",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GameplayTags/DT_GameplayTags_Items_Resources.DT_GameplayTags_Items_Resources",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GameplayTags/DT_GameplayTags_Player.DT_GameplayTags_Player",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GameplayTags/DT_GameplayTags_StanceMultipliers.DT_GameplayTags_StanceMultipliers",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GameplayTags/DT_GamplayTags_EndGameScalingBonusTags.DT_GamplayTags_EndGameScalingBonusTags",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GeneratedItems/DT_Base_Item_Attributes.DT_Base_Item_Attributes",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GeneratedItems/DT_RandomAffixPools_V2.DT_RandomAffixPools_V2",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/GeneratedItems/DT_Yellow_Orange_Modifiers.DT_Yellow_Orange_Modifiers",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/LocalizedDataTables/DT_Attributes.DT_Attributes",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/LocalizedDataTables/DT_StatusEffects.DT_StatusEffects",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/DT_Circle_Round_Skill_Tree_Skills.DT_Circle_Round_Skill_Tree_Skills",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/DT_Crystal_Cards_Skills.DT_Crystal_Cards_Skills",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/DT_Skills_Archery.DT_Skills_Archery",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/DT_Skills_Magery.DT_Skills_Magery",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/DT_Skills_Mauls.DT_Skills_Mauls",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/DT_Skills_OneHand.DT_Skills_OneHand",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/DT_Skills_Scythe.DT_Skills_Scythe",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/DT_Skills_Spear.DT_Skills_Spear",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/DT_Skills_TwoHand.DT_Skills_TwoHand",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/DT_Skills_Unarmed.DT_Skills_Unarmed",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/DT_Stance_Levels.DT_Stance_Levels",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/Localized/DT_Skills_Crafting.DT_Skills_Crafting",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/Localized/DT_Skills_Crafting_S24.DT_Skills_Crafting_S24",
    "/Game/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/Localized/DT_StanceExtendedDescriptions.DT_StanceExtendedDescriptions",
    "/Game/EasySurvivalRPG/Blueprints/Equipment_Modifiers/DT_Equipment_Modifiers.DT_Equipment_Modifiers",
    "/Game/EasySurvivalRPG/Blueprints/InteractionObjects/Fishing/DT_GameplayTags_Items_Fishing.DT_GameplayTags_Items_Fishing",
    "/Game/EasySurvivalRPG/Blueprints/Misc/HomeFortManager/Localized/DT_MapTooltips.DT_MapTooltips",
    "/Game/EasySurvivalRPG/Blueprints/Misc/HomeFortManager/Localized/DelveSystem/DT_UnderrealmGameplayKeys.DT_UnderrealmGameplayKeys",
    "/Game/EasySurvivalRPG/Blueprints/Misc/HomeFortManager/Localized/DelveSystem/DT_UnderrealmRewards.DT_UnderrealmRewards",
    "/Game/EasySurvivalRPG/Blueprints/Misc/HomeFortManager/Localized/DelveSystem/MysteryChest/DT_MysteryAugmentPools.DT_MysteryAugmentPools",
    "/Game/EasySurvivalRPG/Blueprints/Misc/HomeFortManager/Localized/DelveSystem/MysteryChest/DT_UnderrealmMysteryAugmentTags.DT_UnderrealmMysteryAugmentTags",
    "/Game/EasySurvivalRPG/Blueprints/Misc/ItemizationV2/DT_EquipmentAffixes.DT_EquipmentAffixes",
    "/Game/EasySurvivalRPG/Blueprints/Misc/WeaponAttacksSystem/DT_WeaponAttackBonusTags.DT_WeaponAttackBonusTags",
    "/Game/EasySurvivalRPG/Blueprints/Misc/WeaponAttacksSystem/DT_WeaponAttackModifierTags.DT_WeaponAttackModifierTags",
    "/Game/EasySurvivalRPG/Blueprints/Misc/WeaponAttacksSystem/DT_WeaponAttacks.DT_WeaponAttacks",
    "/Game/EasySurvivalRPG/Blueprints/UserInterface/DT_CheatPlayerData.DT_CheatPlayerData",
    "/Game/EasySurvivalRPG/Blueprints/UserInterface/DT_LoadoutLoot.DT_LoadoutLoot",
    "/Game/EasySurvivalRPG/Blueprints/UserInterface/Menus/DT_Char_Create_Names.DT_Char_Create_Names",
    "/Game/EasySurvivalRPG/Blueprints/UserInterface/Menus/Flags/DT_Cultures.DT_Cultures",
    "/Game/EasySurvivalRPG/Blueprints/UserInterface/SkillTree_S3/Data/DT_SkillTree_AttributePresets.DT_SkillTree_AttributePresets",
    "/Game/EasySurvivalRPG/Blueprints/UserInterface/SkillTree_S3/Localized/DT_SkillTree_DropDownCategories.DT_SkillTree_DropDownCategories",
    "/Game/EasySurvivalRPG/Blueprints/UserInterface/SkillTree_S3/SkillDataTables/DT_GENERATED_SkillTree_Main.DT_GENERATED_SkillTree_Main",
    "/Game/EasySurvivalRPG/Blueprints/UserInterface/SkillTree_S3/SkillDataTables/DT_GENERATED_SkillTree_Main_TEST.DT_GENERATED_SkillTree_Main_TEST",
    "/Game/EasySurvivalRPG/Blueprints/UserInterface/Tutorial/Data/DT_OrinTutorialDescriptor.DT_OrinTutorialDescriptor",
    "/Game/EasySurvivalRPG/Blueprints/Zones/DT_Continents.DT_Continents",
    "/Game/EasySurvivalRPG/Blueprints/Zones/DT_CustomMap_Keys.DT_CustomMap_Keys",
    "/Game/EasySurvivalRPG/Blueprints/Zones/DT_CustomRuptureGateDestinations.DT_CustomRuptureGateDestinations",
    "/Game/EasySurvivalRPG/Blueprints/Zones/DT_RuptureRareSpawns.DT_RuptureRareSpawns",
    "/Game/EasySurvivalRPG/Blueprints/Zones/DT_Zones.DT_Zones",
    "/Game/EasySurvivalRPG/Blueprints/Zones/Fortune/DT_GameplayTags_RuptureFortunes.DT_GameplayTags_RuptureFortunes",
    "/Game/EasySurvivalRPG/Blueprints/Zones/Fortune/Data/DT_RuptureFortunes.DT_RuptureFortunes",
    "/Game/EasySurvivalRPG/Pets/DT_Pets.DT_Pets",
    "/Game/EasySurvivalRPG/Skins/DT_Skins.DT_Skins",
    "/Game/ElementalInfusion/DT_ElementalInfusion_Gameplaytags.DT_ElementalInfusion_Gameplaytags",
    "/Game/StylizedCharacter/DT_Item_Customizations_V2.DT_Item_Customizations_V2",
    "/Game/StylizedCharacter/DT_Item_Customizations_V3.DT_Item_Customizations_V3",
    "/Game/StylizedTigerMount/Blueprints/DT_Mounts.DT_Mounts",
}

local done = false
local function Dump(reason)
    if done then return end
    done = true
    print("[DRDump] " .. reason .. ": loading " .. #Tables .. " tables\n")
    ExecuteInGameThread(function()
        for _, path in ipairs(Tables) do
            local ok, err = pcall(LoadAsset, path)
            if not ok then print("[DRDump] load failed " .. path .. " " .. tostring(err) .. "\n") end
        end
        print("[DRDump] tables loaded, dumping usmap\n")
        DumpUSMAP()
        print("[DRDump] usmap done\n")
    end)
end

-- Once the player is in the world, wait a bit for gameplay blueprints to load.
RegisterHook("/Script/Engine.PlayerController:ClientRestart", function()
    ExecuteWithDelay(15000, function() Dump("in-game") end)
end)
-- Fallback if the hook never fires (e.g. stuck in menu): dump after 90 s anyway.
ExecuteWithDelay(90000, function() Dump("timer") end)
