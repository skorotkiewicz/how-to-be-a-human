use rand::prelude::*;
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════
//  Configuration
// ═══════════════════════════════════════════════════════════════

const POPULATION: usize = 24;
const NUM_GROUPS: u32 = 3;
const ROUNDS: usize = 10;
const INTERACTIONS_PER_ROUND: usize = 50;
const GROUP_LABELS: &[&str] = &["α", "β", "γ"];

// ═══════════════════════════════════════════════════════════════
//  Person — stable traits + dynamic social state
// ═══════════════════════════════════════════════════════════════
//
//  Research basis for each axis:
//    empathy     0–100  Baron-Cohen EQ; inverse of Dark Triad callousness
//    altruism    0–1    Continuous dictator-game giving (Engel 2011 meta-analysis)
//    honesty     0–1    HEXACO Honesty-Humility facet (Lee & Ashton 2005)
//    needs_met   1–5    Maslow hierarchy; deficit reduces prosocial *capacity*
//    in_group    0–1    Parochial altruism (Choi & Bowles 2007)
//    forgiveness 0–1    Generous Tit-for-Tat parameter (Nowak & Sigmund 1992)
//    rep_sens    0–1    "Watching eyes" effect magnitude (Bateson et al. 2006)

#[derive(Debug, Clone)]
struct Person {
    id: usize,
    group_id: u32,

    // ── Stable traits ──────────────────────────────────────
    empathy: f32,
    altruism: f32,
    honesty: f32,
    needs_met: u8,
    in_group_bias: f32,
    forgiveness: f32,
    reputation_sensitivity: f32,

    // ── Dynamic state ──────────────────────────────────────
    reputation: f32,
    relationships: HashMap<usize, f32>,

    helped_count: u32,
    refused_count: u32,
    helped_by_count: u32,
    refused_by_count: u32,

    observed_helped: u32,
    observed_refused: u32,
    anonymous_helped: u32,
    anonymous_refused: u32,

    resources: f32,
}

impl Person {
    fn generate(id: usize, rng: &mut impl Rng) -> Self {
        // Weighted needs distribution — few at extremes, most in middle
        let needs = match rng.random_range(0..100) {
            0..=10 => 1,
            11..=30 => 2,
            31..=60 => 3,
            61..=85 => 4,
            _ => 5,
        };
        Person {
            id,
            group_id: rng.random_range(0..NUM_GROUPS),
            empathy: rng.random_range(5.0..95.0),
            altruism: rng.random_range(0.05..0.95),
            honesty: rng.random_range(0.1..1.0),
            needs_met: needs,
            in_group_bias: rng.random_range(0.0..1.0),
            forgiveness: rng.random_range(0.1..1.0),
            reputation_sensitivity: rng.random_range(0.0..1.0),
            reputation: 50.0,
            relationships: HashMap::new(),
            helped_count: 0,
            refused_count: 0,
            helped_by_count: 0,
            refused_by_count: 0,
            observed_helped: 0,
            observed_refused: 0,
            anonymous_helped: 0,
            anonymous_refused: 0,
            resources: 50.0,
        }
    }

    // ── Derived quantities ─────────────────────────────────

    /// Security hypothesis: unmet needs reduce prosocial *capacity*, not virtue.
    /// Level 5 → 100%, 4 → 50%, 3 → 33%, 2 → 25%, 1 → 20%
    fn capacity(&self) -> f32 {
        let deficit = 5 - self.needs_met;
        if deficit == 0 {
            1.0
        } else {
            1.0 / (1.0 + deficit as f32)
        }
    }

    fn effective_empathy(&self) -> f32 {
        self.empathy * self.capacity()
    }

    fn needs_label(&self) -> &'static str {
        match self.needs_met {
            1 => "physiological",
            2 => "safety",
            3 => "belonging",
            4 => "esteem",
            5 => "self-actualize",
            _ => "?",
        }
    }

    fn help_rate(&self) -> f32 {
        let t = self.helped_count + self.refused_count;
        if t == 0 {
            0.0
        } else {
            self.helped_count as f32 / t as f32
        }
    }

    fn observed_help_rate(&self) -> Option<f32> {
        let t = self.observed_helped + self.observed_refused;
        if t == 0 {
            None
        } else {
            Some(self.observed_helped as f32 / t as f32)
        }
    }

    fn anonymous_help_rate(&self) -> Option<f32> {
        let t = self.anonymous_helped + self.anonymous_refused;
        if t == 0 {
            None
        } else {
            Some(self.anonymous_helped as f32 / t as f32)
        }
    }

    // ── Decision models ────────────────────────────────────

    /// Core helping decision combining all six factors.
    ///
    /// willingness = base_altruism × capacity
    ///             + empathy_factor          (Batson 1991)
    ///             + group_factor            (Choi & Bowles 2007)
    ///             + direct_reciprocity      (Axelrod 1984)
    ///             + indirect_reciprocity    (Nowak & Sigmund 1998)
    ///             + observation_boost       (Bateson et al. 2006)
    ///             + benefit_salience        (cost–benefit ratio)
    ///
    /// Help if willingness ≥ cost
    fn decide_help(&self, target: &Person, cost: f32, benefit: f32, observed: bool) -> bool {
        let cap = self.capacity();

        // (1) Base altruism scaled by capacity
        let base = self.altruism * cap;

        // (2) Empathy amplifies concern for the target's welfare
        let emp = (self.effective_empathy() / 100.0) * 0.25;

        // (3) In-group bias: up to 75% reduction for out-group
        let same = self.group_id == target.group_id;
        let grp = if same {
            0.20
        } else {
            0.20 * (1.0 - self.in_group_bias * 0.75)
        };

        // (4) Direct reciprocity — modulated by forgiveness
        //     Positive history always counts; negative history dampened by forgiveness
        let past = self.relationships.get(&target.id).copied().unwrap_or(0.0);
        let weighted_past = if past >= 0.0 {
            past * 0.25
        } else {
            // High forgiveness → grudge fades; low forgiveness → grudge holds
            past * (1.0 - self.forgiveness * 0.6) * 0.25
        };

        // (5) Indirect reciprocity — target's community reputation
        let rep = (target.reputation - 50.0) / 250.0;

        // (6) Observation boost — "watching eyes" effect
        let obs = if observed {
            self.reputation_sensitivity * 0.20
        } else {
            0.0
        };

        // (7) Benefit salience — people help more when impact is large
        //     Empathetic people are more sensitive to benefit magnitude
        let ben = (benefit * 0.05) * (0.5 + self.effective_empathy() / 200.0);

        let willingness = base + emp + grp + weighted_past + rep + obs + ben;
        willingness >= cost
    }

    /// Honesty decision with pro-social deception override.
    ///
    /// A person lies when the cost of truth exceeds their honesty threshold,
    /// EXCEPT when high empathy + vulnerable target → lying to protect
    /// is itself a prosocial act (reverse Dark Triad: Machiavellian
    /// means for empathic ends).
    fn decide_honesty(&self, cost_of_truth: f32, target: &Person) -> bool {
        let honest = self.honesty >= cost_of_truth;
        // Pro-social deception: lying to protect a vulnerable person
        let protecting_vulnerable = target.needs_met <= 2 && cost_of_truth > 0.5;
        let prosocial_lie = protecting_vulnerable && self.effective_empathy() > 60.0;
        honest || prosocial_lie
    }

    // ── State updates ──────────────────────────────────────

    fn record_action(&mut self, helped: bool, cost: f32, observed: bool) {
        if helped {
            self.helped_count += 1;
            self.resources = (self.resources - cost * 8.0).max(0.0);
            if observed {
                self.observed_helped += 1;
            } else {
                self.anonymous_helped += 1;
            }
        } else {
            self.refused_count += 1;
            if observed {
                self.observed_refused += 1;
            } else {
                self.anonymous_refused += 1;
            }
        }
    }

    fn record_received(&mut self, helped: bool, benefit: f32) {
        if helped {
            self.helped_by_count += 1;
            self.resources = (self.resources + benefit * 8.0).min(100.0);
        } else {
            self.refused_by_count += 1;
        }
    }

    fn update_relationship(&mut self, other_id: usize, they_helped: bool) {
        let cur = self.relationships.get(&other_id).copied().unwrap_or(0.0);
        let delta = if they_helped { 0.15 } else { -0.12 };
        self.relationships
            .insert(other_id, (cur + delta).clamp(-1.0, 1.0));
    }

    fn update_reputation(&mut self, helped: bool, observed: bool) {
        if observed {
            let d = if helped { 2.5 } else { -1.8 };
            self.reputation = (self.reputation + d).clamp(0.0, 100.0);
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  Interaction Record
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Interaction {
    round: usize,
    helper_id: usize,
    target_id: usize,
    helped: bool,
    was_honest: bool,
    cost: f32,
    benefit: f32,
    same_group: bool,
    observed: bool,
    prior_relationship: f32, // helper's view of target *before* this interaction
}

// ═══════════════════════════════════════════════════════════════
//  Simulation Engine
// ═══════════════════════════════════════════════════════════════

struct Simulation {
    people: Vec<Person>,
    interactions: Vec<Interaction>,
    rng: StdRng,
}

impl Simulation {
    fn new() -> Self {
        let mut rng = rand::make_rng::<StdRng>();
        let people = (0..POPULATION)
            .map(|id| Person::generate(id, &mut rng))
            .collect();
        Simulation {
            people,
            interactions: Vec::new(),
            rng,
        }
    }

    fn run(&mut self) {
        for round in 0..ROUNDS {
            for _ in 0..INTERACTIONS_PER_ROUND {
                // Pick random pair (distinct)
                let a = self.rng.random_range(0..POPULATION);
                let b = loop {
                    let b = self.rng.random_range(0..POPULATION);
                    if b != a {
                        break b;
                    }
                };

                let cost = self.rng.random_range(0.0..1.0);
                let benefit = self.rng.random_range(0.1..1.0);
                let observed = self.rng.random_bool(0.5);
                let cost_of_truth = self.rng.random_range(0.0..1.0);

                // Snapshot relationship *before* this interaction
                let prior_rel = self.people[a].relationships.get(&b).copied().unwrap_or(0.0);
                let same_group = self.people[a].group_id == self.people[b].group_id;

                let helped = self.people[a].decide_help(&self.people[b], cost, benefit, observed);
                let was_honest = self.people[a].decide_honesty(cost_of_truth, &self.people[b]);

                self.interactions.push(Interaction {
                    round,
                    helper_id: a,
                    target_id: b,
                    helped,
                    was_honest,
                    cost,
                    benefit,
                    same_group,
                    observed,
                    prior_relationship: prior_rel,
                });

                // ── State updates ───────────────────────────
                // Target records whether helper was prosocial
                self.people[b].update_relationship(a, helped);
                // Helper's reputation shifts if observed
                self.people[a].update_reputation(helped, observed);
                // Behavioral counts + resource effects
                self.people[a].record_action(helped, cost, observed);
                self.people[b].record_received(helped, benefit);
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  Analysis & Output
// ═══════════════════════════════════════════════════════════════

fn pct(n: usize, d: usize) -> f32 {
    if d == 0 {
        0.0
    } else {
        n as f32 / d as f32 * 100.0
    }
}

// ─── 0. Population Overview ────────────────────────────────────

fn print_population(sim: &Simulation) {
    println!();
    println!("  ╔═══════════════════════════════════════════════════════════════╗");
    println!("  ║          PROSOCIAL BEHAVIOR SIMULATOR  v2.0                  ║");
    println!("  ║                                                               ║");
    println!("  ║   + Situational Context (anonymous vs observed)               ║");
    println!("  ║   + Reciprocity Dynamics  (direct + indirect, forgiveness)    ║");
    println!("  ║   + In-Group Bias         (parochial altruism)                ║");
    println!("  ║   + Cost Magnitude        (continuous altruism thresholds)    ║");
    println!("  ╚════════════════════════════════════════════════════════════════╝");
    println!();

    for g in 0..NUM_GROUPS {
        let members: Vec<&Person> = sim.people.iter().filter(|p| p.group_id == g).collect();
        let avg_emp = members.iter().map(|p| p.empathy).sum::<f32>() / members.len() as f32;
        let avg_alt = members.iter().map(|p| p.altruism).sum::<f32>() / members.len() as f32;
        let low = members.iter().filter(|p| p.needs_met <= 2).count();
        println!(
            "  Group {} ({} people): avg empathy {:.1}%  avg altruism {:.2}  {} at low needs (≤2)",
            GROUP_LABELS[g as usize],
            members.len(),
            avg_emp,
            avg_alt,
            low
        );
    }

    println!();
    println!("  Trait ranges across population:");
    let minmax = |extractor: fn(&Person) -> f32| -> (f32, f32) {
        let vals: Vec<f32> = sim.people.iter().map(extractor).collect();
        (
            vals.iter().cloned().fold(f32::INFINITY, f32::min),
            vals.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
        )
    };
    let (elo, ehi) = minmax(|p| p.empathy);
    let (alo, ahi) = minmax(|p| p.altruism);
    let (blo, bhi) = minmax(|p| p.in_group_bias);
    let (rlo, rhi) = minmax(|p| p.reputation_sensitivity);
    println!("    Empathy:               {:5.1} – {:.1}", elo, ehi);
    println!("    Altruism:              {:5.2} – {:.2}", alo, ahi);
    println!("    In-group bias:         {:5.2} – {:.2}", blo, bhi);
    println!("    Reputation sensitivity:{:5.2} – {:.2}", rlo, rhi);

    let low_needs = sim.people.iter().filter(|p| p.needs_met <= 2).count();
    println!(
        "    At needs level ≤ 2:    {} / {} ({:.0}%)",
        low_needs,
        POPULATION,
        low_needs as f32 / POPULATION as f32 * 100.0
    );
}

// ─── Round-by-Round ────────────────────────────────────────────

fn print_round_dynamics(sim: &Simulation) {
    println!();
    println!("  ─── Round-by-Round Dynamics ─────────────────────────────────");
    println!();
    println!(
        "  {:5} │ {:>6} │ {:>9} │ {:>9} │ {:>9} │ {:>9}",
        "Rnd", "Help%", "Observed%", "Anonymous%", "SameGrp%", "DiffGrp%"
    );
    println!("  ─────┼────────┼───────────┼───────────┼───────────┼───────────");

    for round in 0..ROUNDS {
        let ri: Vec<&Interaction> = sim
            .interactions
            .iter()
            .filter(|i| i.round == round)
            .collect();
        let total = ri.len();
        let helped = ri.iter().filter(|i| i.helped).count();

        let obs_all: Vec<&&Interaction> = ri.iter().filter(|i| i.observed).collect();
        let obs_h = obs_all.iter().filter(|i| i.helped).count();

        let anon_all: Vec<&&Interaction> = ri.iter().filter(|i| !i.observed).collect();
        let anon_h = anon_all.iter().filter(|i| i.helped).count();

        let same_all: Vec<&&Interaction> = ri.iter().filter(|i| i.same_group).collect();
        let same_h = same_all.iter().filter(|i| i.helped).count();

        let diff_all: Vec<&&Interaction> = ri.iter().filter(|i| !i.same_group).collect();
        let diff_h = diff_all.iter().filter(|i| i.helped).count();

        println!(
            "  {:4} │ {:5.1}% │ {:8.1}% │ {:8.1}% │ {:8.1}% │ {:8.1}%",
            round + 1,
            pct(helped, total),
            pct(obs_h, obs_all.len()),
            pct(anon_h, anon_all.len()),
            pct(same_h, same_all.len()),
            pct(diff_h, diff_all.len()),
        );
    }
}

// ─── 1. Situational Context ────────────────────────────────────

fn print_situational_analysis(sim: &Simulation) {
    println!();
    println!("  ━━ 1. SITUATIONAL CONTEXT (Observation Effect) ━━━━━━━━━━━━");
    println!();
    println!("  The \"watching eyes\" effect (Bateson et al., 2006): people behave");
    println!("  more prosocially when they believe they are being observed.");
    println!("  Individual differences in reputation_sensitivity drive the magnitude.");
    println!();

    let obs: Vec<&Interaction> = sim.interactions.iter().filter(|i| i.observed).collect();
    let anon: Vec<&Interaction> = sim.interactions.iter().filter(|i| !i.observed).collect();
    let obs_help = pct(obs.iter().filter(|i| i.helped).count(), obs.len());
    let anon_help = pct(anon.iter().filter(|i| i.helped).count(), anon.len());

    println!(
        "  When observed:  {:.1}% helped  ({} interactions)",
        obs_help,
        obs.len()
    );
    println!(
        "  When anonymous: {:.1}% helped  ({} interactions)",
        anon_help,
        anon.len()
    );
    println!("  Δ = {:.1} percentage points", obs_help - anon_help);
    println!();

    // Most vs least reputation-sensitive
    let mut by_sens: Vec<&Person> = sim.people.iter().collect();
    by_sens.sort_by(|a, b| {
        b.reputation_sensitivity
            .partial_cmp(&a.reputation_sensitivity)
            .unwrap()
    });

    println!("  Most reputation-sensitive (perform when watched):");
    for p in by_sens.iter().take(3) {
        let o = p
            .observed_help_rate()
            .map(|r| format!("{:.1}%", r * 100.0))
            .unwrap_or_else(|| "N/A".into());
        let a = p
            .anonymous_help_rate()
            .map(|r| format!("{:.1}%", r * 100.0))
            .unwrap_or_else(|| "N/A".into());
        println!(
            "    P{:02} (rep_sens={:.2}): observed={}  anonymous={}",
            p.id + 1,
            p.reputation_sensitivity,
            o,
            a
        );
    }

    println!();
    println!("  Least reputation-sensitive (integrity-driven, consistent):");
    for p in by_sens.iter().rev().take(3) {
        let o = p
            .observed_help_rate()
            .map(|r| format!("{:.1}%", r * 100.0))
            .unwrap_or_else(|| "N/A".into());
        let a = p
            .anonymous_help_rate()
            .map(|r| format!("{:.1}%", r * 100.0))
            .unwrap_or_else(|| "N/A".into());
        println!(
            "    P{:02} (rep_sens={:.2}): observed={}  anonymous={}",
            p.id + 1,
            p.reputation_sensitivity,
            o,
            a
        );
    }
}

// ─── 2. Reciprocity Dynamics ───────────────────────────────────

fn print_reciprocity_analysis(sim: &Simulation) {
    println!();
    println!("  ━━ 2. RECIPROCITY DYNAMICS ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("  Direct reciprocity (Axelrod, 1984): you help those who helped you.");
    println!("  Indirect reciprocity (Nowak & Sigmund, 1998): you help those with");
    println!("  good community reputation. Forgiveness modulates grudge-holding —");
    println!("  generous tit-for-tat outperforms strict tit-for-tat in noisy worlds.");
    println!();

    // Help rate by prior relationship
    let pos: Vec<&Interaction> = sim
        .interactions
        .iter()
        .filter(|i| i.prior_relationship > 0.1)
        .collect();
    let neg: Vec<&Interaction> = sim
        .interactions
        .iter()
        .filter(|i| i.prior_relationship < -0.1)
        .collect();
    let neut: Vec<&Interaction> = sim
        .interactions
        .iter()
        .filter(|i| i.prior_relationship >= -0.1 && i.prior_relationship <= 0.1)
        .collect();

    println!("  Help rate by prior relationship with target:");
    println!(
        "    Positive history (they helped me):   {:.1}%  ({} interactions)",
        pct(pos.iter().filter(|i| i.helped).count(), pos.len()),
        pos.len()
    );
    println!(
        "    Neutral / no history:                {:.1}%  ({} interactions)",
        pct(neut.iter().filter(|i| i.helped).count(), neut.len()),
        neut.len()
    );
    println!(
        "    Negative history (they refused me):  {:.1}%  ({} interactions)",
        pct(neg.iter().filter(|i| i.helped).count(), neg.len()),
        neg.len()
    );
    println!();

    // Temporal dynamics: early vs late rounds
    let early: Vec<&Interaction> = sim
        .interactions
        .iter()
        .filter(|i| i.round < ROUNDS / 2)
        .collect();
    let late: Vec<&Interaction> = sim
        .interactions
        .iter()
        .filter(|i| i.round >= ROUNDS / 2)
        .collect();
    let early_pct = pct(early.iter().filter(|i| i.helped).count(), early.len());
    let late_pct = pct(late.iter().filter(|i| i.helped).count(), late.len());

    println!("  Temporal shift (reciprocity networks forming):");
    println!(
        "    Early rounds (1–{}):  {:.1}% helped",
        ROUNDS / 2,
        early_pct
    );
    println!(
        "    Late rounds  ({}–{}): {:.1}% helped",
        ROUNDS / 2 + 1,
        ROUNDS,
        late_pct
    );
    println!(
        "    Shift: {:+.1} pp  (positive = reciprocity building trust)",
        late_pct - early_pct
    );
    println!();

    // Forgiveness: helping despite a grudge
    let mut by_forg: Vec<&Person> = sim.people.iter().collect();
    by_forg.sort_by(|a, b| b.forgiveness.partial_cmp(&a.forgiveness).unwrap());

    println!("  Most forgiving (generous tit-for-tat):");
    for p in by_forg.iter().take(2) {
        let neg_i: Vec<&Interaction> = sim
            .interactions
            .iter()
            .filter(|i| i.helper_id == p.id && i.prior_relationship < -0.1)
            .collect();
        let helped_any = pct(neg_i.iter().filter(|i| i.helped).count(), neg_i.len());
        println!(
            "    P{:02} (forgiveness={:.2}): helped despite grudge {:.1}% of the time",
            p.id + 1,
            p.forgiveness,
            helped_any
        );
    }
    println!("  Least forgiving (grudge-holders):");
    for p in by_forg.iter().rev().take(2) {
        let neg_i: Vec<&Interaction> = sim
            .interactions
            .iter()
            .filter(|i| i.helper_id == p.id && i.prior_relationship < -0.1)
            .collect();
        let helped_any = pct(neg_i.iter().filter(|i| i.helped).count(), neg_i.len());
        println!(
            "    P{:02} (forgiveness={:.2}): helped despite grudge {:.1}% of the time",
            p.id + 1,
            p.forgiveness,
            helped_any
        );
    }
    println!();

    // Cooperative pairs — mutual positive relationships
    let mut pairs = Vec::new();
    for a in 0..sim.people.len() {
        for b in (a + 1)..sim.people.len() {
            let ab = sim.people[a].relationships.get(&b).copied().unwrap_or(0.0);
            let ba = sim.people[b].relationships.get(&a).copied().unwrap_or(0.0);
            if ab > 0.2 && ba > 0.2 {
                pairs.push((a, b, ab, ba));
            }
        }
    }
    pairs.sort_by(|x, y| (y.2 + y.3).partial_cmp(&(x.2 + x.3)).unwrap());

    println!("  Cooperative pairs (mutual positive relationships):");
    if pairs.is_empty() {
        println!("    None emerged — interactions too sparse or hostile.");
    } else {
        for (a, b, ab, ba) in pairs.iter().take(5) {
            let same = sim.people[*a].group_id == sim.people[*b].group_id;
            println!(
                "    P{:02} ↔ P{:02}: trust={:.2}/{:.2}  {}",
                a + 1,
                b + 1,
                ab,
                ba,
                if same {
                    "(same group)"
                } else {
                    "(cross-group!)"
                }
            );
        }
        let cross = pairs
            .iter()
            .filter(|(a, b, _, _)| sim.people[*a].group_id != sim.people[*b].group_id)
            .count();
        println!(
            "    Total: {} pairs, {} cross-group ({:.0}%)",
            pairs.len(),
            cross,
            if !pairs.is_empty() {
                cross as f32 / pairs.len() as f32 * 100.0
            } else {
                0.0
            }
        );
    }
}

// ─── 3. In-Group Bias ──────────────────────────────────────────

fn print_ingroup_analysis(sim: &Simulation) {
    println!();
    println!("  ━━ 3. IN-GROUP BIAS (Parochial Altruism) ━━━━━━━━━━━━━━━━━━");
    println!();
    println!("  Parochial altruism (Choi & Bowles, 2007): willingness to incur");
    println!("  cost for in-group members exceeds that for out-group. The ratio");
    println!("  varies enormously between individuals — from universalists who");
    println!("  make no distinction to tribalists who barely help out-group at all.");
    println!();

    let same: Vec<&Interaction> = sim.interactions.iter().filter(|i| i.same_group).collect();
    let diff: Vec<&Interaction> = sim.interactions.iter().filter(|i| !i.same_group).collect();
    let same_help = pct(same.iter().filter(|i| i.helped).count(), same.len());
    let diff_help = pct(diff.iter().filter(|i| i.helped).count(), diff.len());
    let ratio = if diff_help > 0.0 {
        same_help / diff_help
    } else {
        f32::INFINITY
    };

    println!(
        "  Same-group interactions:  {:.1}% helped  ({} total)",
        same_help,
        same.len()
    );
    println!(
        "  Different-group:          {:.1}% helped  ({} total)",
        diff_help,
        diff.len()
    );
    println!("  In-group favoritism ratio: {:.2} : 1", ratio);
    println!();

    // Cost interaction: in-group bias amplifies at higher cost
    let buckets = [
        ("Trivial   (cost ≤ 0.3)", 0.0, 0.3),
        ("Moderate  (0.3 < cost ≤ 0.6)", 0.3, 0.6),
        ("High      (cost > 0.6)", 0.6, 1.01),
    ];
    println!("  In-group favoritism by cost level:");
    println!(
        "    {:30} │ {:>9} │ {:>9} │ {:>5}",
        "Cost", "Same-grp%", "Diff-grp%", "Ratio"
    );
    println!("    ───────────────────────────────┼───────────┼───────────┼──────");
    for (label, lo, hi) in &buckets {
        let s: Vec<&Interaction> = same
            .iter()
            .copied()
            .filter(|i| i.cost >= *lo && i.cost < *hi)
            .collect();
        let d: Vec<&Interaction> = diff
            .iter()
            .copied()
            .filter(|i| i.cost >= *lo && i.cost < *hi)
            .collect();
        let s_pct = pct(s.iter().filter(|i| i.helped).count(), s.len());
        let d_pct = pct(d.iter().filter(|i| i.helped).count(), d.len());
        let r = if d_pct > 0.0 {
            s_pct / d_pct
        } else {
            f32::INFINITY
        };
        println!(
            "    {:30} │ {:8.1}% │ {:8.1}% │ {:4.2}:1",
            label, s_pct, d_pct, r
        );
    }

    println!();

    // Most tribal vs most universal
    let mut by_bias: Vec<&Person> = sim.people.iter().collect();
    by_bias.sort_by(|a, b| b.in_group_bias.partial_cmp(&a.in_group_bias).unwrap());

    println!("  Most tribal (high in-group bias):");
    for p in by_bias.iter().take(3) {
        let s_i: Vec<&Interaction> = sim
            .interactions
            .iter()
            .filter(|i| i.helper_id == p.id && i.same_group)
            .collect();
        let d_i: Vec<&Interaction> = sim
            .interactions
            .iter()
            .filter(|i| i.helper_id == p.id && !i.same_group)
            .collect();
        let s = pct(s_i.iter().filter(|i| i.helped).count(), s_i.len());
        let d = pct(d_i.iter().filter(|i| i.helped).count(), d_i.len());
        println!(
            "    P{:02} (bias={:.2}): in-group {:.1}%  out-group {:.1}%  gap {:.1}pp",
            p.id + 1,
            p.in_group_bias,
            s,
            d,
            s - d
        );
    }
    println!();
    println!("  Most universal (low in-group bias):");
    for p in by_bias.iter().rev().take(3) {
        let s_i: Vec<&Interaction> = sim
            .interactions
            .iter()
            .filter(|i| i.helper_id == p.id && i.same_group)
            .collect();
        let d_i: Vec<&Interaction> = sim
            .interactions
            .iter()
            .filter(|i| i.helper_id == p.id && !i.same_group)
            .collect();
        let s = pct(s_i.iter().filter(|i| i.helped).count(), s_i.len());
        let d = pct(d_i.iter().filter(|i| i.helped).count(), d_i.len());
        println!(
            "    P{:02} (bias={:.2}): in-group {:.1}%  out-group {:.1}%  gap {:.1}pp",
            p.id + 1,
            p.in_group_bias,
            s,
            d,
            s - d
        );
    }
}

// ─── 4. Cost Magnitude ─────────────────────────────────────────

fn print_cost_analysis(sim: &Simulation) {
    println!();
    println!("  ━━ 4. COST MAGITUDE (Continuous Altruism Thresholds) ━━━━━━━");
    println!();
    println!("  Altruism isn't binary — it depends on cost (Fehr & Schmidt, 1999).");
    println!("  Even generous people have limits. The model treats altruism as a");
    println!("  continuous threshold: willingness must exceed cost for helping");
    println!("  to occur. This produces a natural decay curve, not a step function.");
    println!();

    let buckets = [
        ("Trivial      (0.00–0.20)", 0.0, 0.2),
        ("Low          (0.20–0.40)", 0.2, 0.4),
        ("Moderate     (0.40–0.60)", 0.4, 0.6),
        ("Significant  (0.60–0.80)", 0.6, 0.8),
        ("Severe       (0.80–1.00)", 0.8, 1.01),
    ];

    println!(
        "  {:28} │ {:>5} │ {:>6} │ {:>6}",
        "Cost Level", "N", "Helped", "Rate"
    );
    println!("  ─────────────────────────────┼───────┼────────┼───────");

    for (label, lo, hi) in &buckets {
        let b: Vec<&Interaction> = sim
            .interactions
            .iter()
            .filter(|i| i.cost >= *lo && i.cost < *hi)
            .collect();
        let h = b.iter().filter(|i| i.helped).count();
        println!(
            "  {:28} │ {:5} │ {:6} │ {:5.1}%",
            label,
            b.len(),
            h,
            pct(h, b.len())
        );
    }

    println!();

    // Even the most altruistic have limits
    let most_alt = sim
        .people
        .iter()
        .max_by(|a, b| a.altruism.partial_cmp(&b.altruism).unwrap())
        .unwrap();
    let hc: Vec<&Interaction> = sim
        .interactions
        .iter()
        .filter(|i| i.helper_id == most_alt.id && i.cost > 0.6)
        .collect();
    let hc_helped = pct(hc.iter().filter(|i| i.helped).count(), hc.len());
    let lc: Vec<&Interaction> = sim
        .interactions
        .iter()
        .filter(|i| i.helper_id == most_alt.id && i.cost <= 0.3)
        .collect();
    let lc_helped = pct(lc.iter().filter(|i| i.helped).count(), lc.len());

    println!(
        "  Most altruistic person: P{:02} (altruism={:.2}, empathy={:.1}%)",
        most_alt.id + 1,
        most_alt.altruism,
        most_alt.empathy
    );
    println!("    Low cost (≤0.3):    {:.1}% helped", lc_helped);
    println!("    High cost (>0.6):   {:.1}% helped", hc_helped);
    println!("    → Even the most generous have cost thresholds.");

    println!();

    // Capacity interaction: people with unmet needs are cost-sensitive
    let low_needs: Vec<&Person> = sim.people.iter().filter(|p| p.needs_met <= 2).collect();
    let high_needs: Vec<&Person> = sim.people.iter().filter(|p| p.needs_met >= 4).collect();

    let low_cost_rate = |people: &[&Person]| -> f32 {
        let ids: Vec<usize> = people.iter().map(|p| p.id).collect();
        let ints: Vec<&Interaction> = sim
            .interactions
            .iter()
            .filter(|i| ids.contains(&i.helper_id))
            .collect();
        let hi: Vec<&&Interaction> = ints.iter().filter(|i| i.cost > 0.5).collect();
        pct(hi.iter().filter(|i| i.helped).count(), hi.len())
    };

    if !low_needs.is_empty() && !high_needs.is_empty() {
        println!("  Security hypothesis × cost:");
        println!(
            "    Low needs (≤2), high cost (>0.5): {:.1}% helped",
            low_cost_rate(&low_needs)
        );
        println!(
            "    High needs (≥4), high cost (>0.5): {:.1}% helped",
            low_cost_rate(&high_needs)
        );
        println!("    → Unmet needs amplify cost sensitivity. The poor pay more,");
        println!("      but can afford less. Capacity shrinks the willingness envelope.");
    }
}

// ─── 5. Individual Spotlights ──────────────────────────────────

fn print_spotlights(sim: &Simulation) {
    println!();
    println!("  ━━ Individual Spotlights ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // ── THE SAINT: high altruism + empathy, low reputation sensitivity ──
    let mut saints: Vec<&Person> = sim.people.iter().collect();
    saints.sort_by(|a, b| {
        let sa =
            a.altruism * 0.4 + (a.empathy / 100.0) * 0.3 + (1.0 - a.reputation_sensitivity) * 0.3;
        let sb =
            b.altruism * 0.4 + (b.empathy / 100.0) * 0.3 + (1.0 - b.reputation_sensitivity) * 0.3;
        sb.partial_cmp(&sa).unwrap()
    });
    let saint = saints[0];
    println!("  💎 THE SAINT — P{:02}", saint.id + 1);
    println!(
        "    Empathy {:.1}%  Altruism {:.2}  Honesty {:.2}",
        saint.empathy, saint.altruism, saint.honesty
    );
    println!(
        "    Reputation sensitivity {:.2} (low → integrity-driven)",
        saint.reputation_sensitivity
    );
    println!(
        "    Needs: {} ({})  Capacity: {:.0}%",
        saint.needs_met,
        saint.needs_label(),
        saint.capacity() * 100.0
    );
    println!("    Help rate: {:.1}% overall", saint.help_rate() * 100.0);
    println!("    → \"Helps because it's right, not because someone's watching.\"");
    println!();

    // ── THE SURVIVOR: high character but low needs ──
    let mut survivors: Vec<&Person> = sim.people.iter().collect();
    survivors.sort_by(|a, b| {
        // High character, low needs = big gap between potential and reality
        let score_a =
            (a.altruism + a.honesty + a.empathy / 100.0) / 3.0 * (6.0 - a.needs_met as f32); // weight by need deficit
        let score_b =
            (b.altruism + b.honesty + b.empathy / 100.0) / 3.0 * (6.0 - b.needs_met as f32);
        score_b.partial_cmp(&score_a).unwrap()
    });
    let survivor = survivors[0];
    println!("  🌱 THE SURVIVOR — P{:02}", survivor.id + 1);
    println!(
        "    Empathy {:.1}%  Altruism {:.2}  Honesty {:.2}",
        survivor.empathy, survivor.altruism, survivor.honesty
    );
    println!(
        "    Needs: {} ({})  Capacity: {:.0}%",
        survivor.needs_met,
        survivor.needs_label(),
        survivor.capacity() * 100.0
    );
    println!(
        "    Help rate: {:.1}% overall",
        survivor.help_rate() * 100.0
    );
    println!(
        "    Potential empathy: {:.1}%  Effective empathy: {:.1}%  Lost: {:.1}pp",
        survivor.empathy,
        survivor.effective_empathy(),
        survivor.empathy - survivor.effective_empathy()
    );
    println!("    → \"Wants to help. Can't always afford to. Character ≠ capacity.\"");
    println!();

    // ── THE PERFORMER: high reputation sensitivity, moderate altruism ──
    let mut performers: Vec<&Person> = sim.people.iter().collect();
    performers.sort_by(|a, b| {
        let sa = a.reputation_sensitivity * 2.0 - a.altruism;
        let sb = b.reputation_sensitivity * 2.0 - b.altruism;
        sb.partial_cmp(&sa).unwrap()
    });
    let performer = performers[0];
    println!("  🎭 THE PERFORMER — P{:02}", performer.id + 1);
    println!(
        "    Empathy {:.1}%  Altruism {:.2}  Honesty {:.2}",
        performer.empathy, performer.altruism, performer.honesty
    );
    println!(
        "    Reputation sensitivity {:.2} (high → audience-driven)",
        performer.reputation_sensitivity
    );
    let obs_r = performer
        .observed_help_rate()
        .map(|r| format!("{:.1}%", r * 100.0))
        .unwrap_or_else(|| "N/A".into());
    let anon_r = performer
        .anonymous_help_rate()
        .map(|r| format!("{:.1}%", r * 100.0))
        .unwrap_or_else(|| "N/A".into());
    println!(
        "    Observed help rate: {}  Anonymous help rate: {}",
        obs_r, anon_r
    );
    println!("    → \"Helps when watched. The audience is the conscience.\"");
    println!();

    // ── THE TRIBALIST: high in-group bias ──
    let mut tribalists: Vec<&Person> = sim.people.iter().collect();
    tribalists.sort_by(|a, b| b.in_group_bias.partial_cmp(&a.in_group_bias).unwrap());
    let tribalist = tribalists[0];
    println!("  🏴 THE TRIBALIST — P{:02}", tribalist.id + 1);
    println!(
        "    Empathy {:.1}%  Altruism {:.2}  In-group bias {:.2}",
        tribalist.empathy, tribalist.altruism, tribalist.in_group_bias
    );
    let s_i: Vec<&Interaction> = sim
        .interactions
        .iter()
        .filter(|i| i.helper_id == tribalist.id && i.same_group)
        .collect();
    let d_i: Vec<&Interaction> = sim
        .interactions
        .iter()
        .filter(|i| i.helper_id == tribalist.id && !i.same_group)
        .collect();
    let s = pct(s_i.iter().filter(|i| i.helped).count(), s_i.len());
    let d = pct(d_i.iter().filter(|i| i.helped).count(), d_i.len());
    println!(
        "    In-group help rate: {:.1}%  Out-group help rate: {:.1}%",
        s, d
    );
    println!("    → \"Loyal to us. Suspicious of them. Parochial altruism.\"");
    println!();

    // ── THE UNIVERSALIST: low in-group bias, high altruism ──
    let mut universalists: Vec<&Person> = sim.people.iter().collect();
    universalists.sort_by(|a, b| {
        let sa = (1.0 - a.in_group_bias) * 0.6 + a.altruism * 0.4;
        let sb = (1.0 - b.in_group_bias) * 0.6 + b.altruism * 0.4;
        sb.partial_cmp(&sa).unwrap()
    });
    let universalist = universalists[0];
    println!("  🌍 THE UNIVERSALIST — P{:02}", universalist.id + 1);
    println!(
        "    Empathy {:.1}%  Altruism {:.2}  In-group bias {:.2}",
        universalist.empathy, universalist.altruism, universalist.in_group_bias
    );
    let s_i: Vec<&Interaction> = sim
        .interactions
        .iter()
        .filter(|i| i.helper_id == universalist.id && i.same_group)
        .collect();
    let d_i: Vec<&Interaction> = sim
        .interactions
        .iter()
        .filter(|i| i.helper_id == universalist.id && !i.same_group)
        .collect();
    let s = pct(s_i.iter().filter(|i| i.helped).count(), s_i.len());
    let d = pct(d_i.iter().filter(|i| i.helped).count(), d_i.len());
    println!(
        "    In-group help rate: {:.1}%  Out-group help rate: {:.1}%",
        s, d
    );
    println!("    → \"A stranger is just a friend you haven't met yet.\"");
}

// ─── Model Notes ───────────────────────────────────────────────

fn print_model_notes() {
    println!();
    println!("  ━━ Model Architecture ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("  DECIDE_HELP:  willingness ≥ cost  →  help");
    println!();
    println!("  willingness = altruism × capacity        (base prosocial drive)");
    println!("              + empathy_factor              (Batson 1991)");
    println!("              + group_factor                (Choi & Bowles 2007)");
    println!("              + direct_reciprocity          (Axelrod 1984)");
    println!("              + indirect_reciprocity        (Nowak & Sigmund 1998)");
    println!("              + observation_boost           (Bateson et al. 2006)");
    println!("              + benefit_salience            (cost–benefit ratio)");
    println!();
    println!("  DECIDE_HONESTY:  honesty ≥ cost_of_truth  →  tell truth");
    println!("    override:  empathy > 60% AND target vulnerable → lie to protect");
    println!();
    println!("  KEY INSIGHT: Goodness is not a trait. It's a function of:");
    println!("    WHO you are     (stable traits: empathy, altruism, honesty)");
    println!("    WHERE you are   (needs met: capacity multiplier)");
    println!("    WHO you're with (in-group vs out-group)");
    println!("    WHO's watching  (observed vs anonymous)");
    println!("    WHAT it costs   (continuous threshold, not binary)");
    println!("    WHAT HAPPENED   (reciprocity history + forgiveness)");
    println!();
    println!("  The same person can be a saint in one context and indifferent");
    println!("  in another. That's not hypocrisy — it's the architecture of");
    println!("  human prosociality. Understanding it is the first step toward");
    println!("  designing systems that bring out the best in everyone.");
    println!();
}

// ═══════════════════════════════════════════════════════════════
//  Main
// ═══════════════════════════════════════════════════════════════

fn main() {
    let mut sim = Simulation::new();

    print_population(&sim);

    println!();
    println!(
        "  Running {} rounds × {} interactions/round = {} total...",
        ROUNDS,
        INTERACTIONS_PER_ROUND,
        ROUNDS * INTERACTIONS_PER_ROUND
    );
    println!();

    sim.run();

    let total_helped = sim.interactions.iter().filter(|i| i.helped).count();
    let total = sim.interactions.len();
    println!(
        "  Result: {} / {} interactions were prosocial ({:.1}%)",
        total_helped,
        total,
        pct(total_helped, total)
    );

    print_round_dynamics(&sim);
    print_situational_analysis(&sim);
    print_reciprocity_analysis(&sim);
    print_ingroup_analysis(&sim);
    print_cost_analysis(&sim);
    print_spotlights(&sim);
    print_model_notes();
}
