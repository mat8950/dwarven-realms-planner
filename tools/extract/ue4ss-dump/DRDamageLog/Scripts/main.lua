-- Logs every hit the training dummy takes (BP_Building_Dummy.DamageTaken) to
-- ue4ss/DRDamageLog.csv: game time, damage causer (actor instance and class), damage,
-- damage type and crit flag. tools/validate/damagelog.py turns it into hits per proc
-- for each offhand ability, to calibrate the planner's DPS.

local UEHelpers = require("UEHelpers")

local DAMAGE = "Damage_2_8B27BFA04F0254A2EB2C86BA022756E4"
local TYPE = "DamageTypeClass_17_08410F564C9DF3B644E95898C60FCE88"
local CRIT = "IsCriticalHit_34_6CAD2CC644A692989D048D82F10DC118"
local HOOK = "/Game/EasySurvivalRPG/Blueprints/InteractionObjects/BuildingObjects/Placeable/BP_Building_Dummy.BP_Building_Dummy_C:DamageTaken"

local path = "ue4ss/DRDamageLog.csv"
local file = io.open(path, "a")
if not file then
    path = "DRDamageLog.csv"
    file = io.open(path, "a")
end
file:write("time,causer,class,damage,type,crit\n")
file:flush()

local function safe(f, default)
    local ok, v = pcall(f)
    if ok and v ~= nil then return v end
    return default
end

local function shortName(obj)
    return safe(function() return obj:GetFName():ToString() end, "?")
end

local function className(obj)
    return safe(function() return obj:GetClass():GetFName():ToString() end, "?")
end

RegisterHook(HOOK, function(self, data, instigatedBy, causer, hit)
    local dummy = self:get()
    local d = data:get()
    local c = causer:get()
    local time = safe(function() return UEHelpers.GetKismetSystemLibrary():GetGameTimeInSeconds(dummy) end, 0)
    local damage = safe(function() return d[DAMAGE] end, 0)
    local dtype = safe(function() return d[TYPE]:GetFName():ToString() end, "?")
    local crit = safe(function() return d[CRIT] end, false)
    local valid = safe(function() return c:IsValid() end, false)
    file:write(string.format("%.4f,%s,%s,%.9g,%s,%s\n", time, valid and shortName(c) or "none", valid and className(c) or "none",
        damage, dtype, tostring(crit)))
    file:flush()
end)

print("[DRDamageLog] logging dummy hits to " .. path .. "\n")
