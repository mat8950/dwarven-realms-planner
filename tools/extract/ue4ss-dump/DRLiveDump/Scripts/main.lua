-- Dumps the live player's attribute arrays to ue4ss/DRLiveDump.json, to validate
-- the planner's stat engine against the real game. Runs 20 s after the player
-- spawns, and again on Ctrl+F7.

local TAG = "GameplayTag_2_9682C35B48A5160809F4BFB3A6AECB2C"
local VALUE = "Value_5_43BD33CA455ADE70795B28BCBCDB22CD"

-- Live components only (skip class defaults), player's first. Matching on the full
-- object path ("...BP_Character_Player_C_123.AttributesComponent") needs no calls.
local function liveComps(class)
    local player, others = {}, {}
    for _, c in ipairs(FindAllOf(class) or {}) do
        if c:IsValid() then
            local name = c:GetFullName()
            if not name:find("Default__", 1, true) then
                if name:find("BP_Character_Player", 1, true) then
                    player[#player + 1] = c
                else
                    others[#others + 1] = c
                end
            end
        end
    end
    for _, c in ipairs(others) do player[#player + 1] = c end
    return player
end

local function findPlayerComp(class)
    return liveComps(class)[1]
end

local function attrEntry(s)
    local tag = s[TAG].TagName:ToString()
    return string.format('{"tag":"%s","value":%.9g}', tag, s[VALUE])
end

local function dumpArray(arr)
    local out = {}
    if not arr then return "[]" end
    arr:ForEach(function(_, elem)
        local ok, s = pcall(function() return attrEntry(elem:get()) end)
        if ok then out[#out + 1] = s end
    end)
    return "[" .. table.concat(out, ",") .. "]"
end

local function dumpMap(map)
    local out = {}
    if not map then return "[]" end
    map:ForEach(function(_, value)
        local ok, s = pcall(function() return attrEntry(value:get()) end)
        if ok then out[#out + 1] = s end
    end)
    return "[" .. table.concat(out, ",") .. "]"
end

local function num(obj, prop)
    local ok, v = pcall(function() return obj[prop] end)
    if ok and type(v) == "number" then return string.format("%.9g", v) end
    return "null"
end

local function dumpComp(comp)
    local parts = {
        '"name":"' .. comp:GetFullName():gsub('"', "'") .. '"',
        '"BaseAttributes":' .. dumpArray(comp.BaseAttributes),
        '"PlayerAttributes":' .. dumpArray(comp.PlayerAttributes),
        '"EquipmentAttributes":' .. dumpArray(comp.EquipmentAttributes),
        '"EffectAttributes":' .. dumpArray(comp.EffectAttributes),
        '"SelectedItemAttributes":' .. dumpArray(comp.SelectedItemAttributes),
        '"AttributesNamed":' .. dumpMap(comp.AttributesNamed),
        '"StanceDamageMultiplier":' .. num(comp, "Stance Damage Multiplier"),
        '"StanceCritMult":' .. num(comp, "StanceCritMult"),
    }
    return "{" .. table.concat(parts, ",") .. "}"
end

local function Dump(reason)
    local comps = liveComps("BP_AttributesComponent_C")
    print("[DRLiveDump] " .. #comps .. " attribute components\n")
    if #comps == 0 then return end
    local list = {}
    for i, c in ipairs(comps) do
        if i > 4 then break end
        local ok, s = pcall(dumpComp, c)
        if ok then list[#list + 1] = s else print("[DRLiveDump] dump failed: " .. tostring(s) .. "\n") end
    end
    local extra = {}
    local stance = findPlayerComp("BP_PlayerStanceSkillManagerComponent_C")
    if stance then extra[#extra + 1] = '"CurrentStance":' .. num(stance, "CurrentStance") end
    local potions = findPlayerComp("BP_PotionManager_C")
    if potions then
        extra[#extra + 1] = '"AvailablePotions":' .. num(potions, "Available Potions")
        extra[#extra + 1] = '"MaxPotions":' .. num(potions, "Max Potions")
    end
    extra[#extra + 1] = '"reason":"' .. reason .. '"'
    local f = io.open("ue4ss/DRLiveDump.json", "w") or io.open("DRLiveDump.json", "w")
    f:write("{" .. table.concat(extra, ",") .. ',"components":[' .. table.concat(list, ",") .. "]}")
    f:close()
    print("[DRLiveDump] written (" .. reason .. ")\n")
end

RegisterHook("/Script/Engine.PlayerController:ClientRestart", function()
    ExecuteWithDelay(20000, function() ExecuteInGameThread(function() Dump("spawn") end) end)
end)

RegisterKeyBind(Key.F7, { ModifierKey.CONTROL }, function()
    ExecuteInGameThread(function() Dump("key") end)
end)
