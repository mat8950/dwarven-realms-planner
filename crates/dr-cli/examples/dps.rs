fn main() -> anyhow::Result<()> {
    let root = std::env::var_os("DR_PLANNER_ROOT").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let gd = dr_core::gamedata::GameData::load(&root, "fr")?;
    let cat = dr_core::catalog::Catalog::build(&gd)?;
    let mut p = dr_core::planner::Planner::new(&gd, &cat)?;
    p.open(std::env::args().nth(1).unwrap().as_ref())?;
    let s = p.compute(&Default::default())?;
    let d = &s.dps;
    println!("CR {:.0} boss x{:.3} crit {:.2} ASbonus {:.3} castspeed {:.3} regen {:.1}", s.combat_rating, d.boss_mult, d.crit_chance, d.attack_speed_bonus, d.cast_speed, d.energy_regen);
    for (a, x) in cat.attacks.iter().zip(&d.attacks).filter(|(a, _)| a.stance == s.stance) {
        println!("  {:<22} hit {:>12.0} crit {:>12.0} avg {:>12.0} t {:.3}s rate {:.2}/{:.2} cost {:.0} dps {:>14.0} boss {:>14.0}", a.name, x.hit, x.crit, x.avg, x.cast_time, x.burst_rate, x.sustained_rate, x.energy_cost, x.dps, x.dps_boss);
    }
    for (a, x) in cat.abilities.iter().zip(&d.abilities).filter(|(_, x)| x.offhands > 0) {
        println!("  {:<22} proc {:>14.0} x{} n{} cd {:?} cycle {:?} dps {:>14.0} boss {:>14.0} | dummy 5s {:.4e} boss {:.4e}", a.name, x.per_proc, x.hits, x.offhands, x.cooldown, x.cycle, x.dps, x.dps_boss, x.meter, x.meter_boss);
    }
    for b in &d.basic { println!("  basic stance {} hits {} cycle {:.3}s rate {:.2}/s dps {:.0} boss {:.0}", b.stance, b.hits, b.cycle_time, b.rate, b.dps, b.dps_boss); }
    Ok(())
}
