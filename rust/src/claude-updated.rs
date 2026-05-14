use rand::prelude::*;

/// Core prosocial traits — inverse Dark Triad predictors
///
/// References:
///   • HEXACO Honesty-Humility as strongest Dark Triad predictor (Lee & Ashton, 2005)
///   • Empathy-altruism hypothesis (Batson, 1991)
///   • Dictator game meta-analysis: ~60% show altruistic giving (Engel, 2011)
///   • Security hypothesis: unmet needs reduce prosocial capacity (Brown et al., 2022)
#[derive(Debug)]
struct ProsocialProfile {
    /// 0.0–100.0: ability to understand and share others' feelings
    empathy: f32,

    /// Helps others even at personal cost
    altruistic: bool,

    /// Defaults to truth; lies only to protect vulnerable people
    honest_by_default: bool,

    /// Maslow level (1–5): physiological → safety → belonging → esteem → self-actualization
    needs_met_level: u8,

    /// How embedded in mutual relationships this person is.
    /// Distinct from altruism: someone can be altruistic but isolated,
    /// or deeply interdependent without being conventionally selfless.
    /// Scale 0.0–100.0.
    relational_embeddedness: f32,
}

impl ProsocialProfile {
    fn simulate(rng: &mut impl Rng) -> Self {
        ProsocialProfile {
            empathy: rng.random_range(0.0..100.0),
            altruistic: rng.random_bool(0.6),
            honest_by_default: rng.random_bool(0.75),
            needs_met_level: rng.random_range(1..=5),
            relational_embeddedness: rng.random_range(0.0..100.0),
        }
    }

    /// The core behavioral model:
    ///   1. Meet your own needs first (Maslow)
    ///   2. Contribute back proportionally to remaining capacity
    ///   3. Interdependence is structural — it reflects existing bonds,
    ///      not just current willingness to help
    fn behavior_output(&self) -> BehaviorOutput {
        let need_deficit = 5 - self.needs_met_level;

        // Unmet needs shrink prosocial bandwidth — not willingness, capacity
        let available_capacity = if need_deficit == 0 {
            1.0
        } else {
            1.0 / (1.0 + need_deficit as f32)
        };

        let effective_empathy = self.empathy * available_capacity;

        // Can't consistently help others if your own foundation is shaky
        let likely_to_help = self.altruistic && (need_deficit <= 2);

        // High empathy can compensate for situational dishonesty
        // (e.g., lying to protect someone — prosocial deception)
        let likely_truthful = self.honest_by_default || (effective_empathy > 65.0);

        // Interdependence is distinct from helping behavior.
        // A person can be deeply connected to others even when they currently
        // lack the capacity to help — bonds persist through hard periods.
        // Relational embeddedness carries the structural weight here;
        // empathy and capacity modulate the quality of that connection.
        let embeddedness_factor = self.relational_embeddedness / 100.0;
        let quality_factor = (effective_empathy / 100.0) * available_capacity;
        let interdependence = (embeddedness_factor * 0.6 + quality_factor * 0.4) * 100.0;

        BehaviorOutput {
            effective_empathy,
            likely_to_help,
            likely_truthful,
            interdependence,
        }
    }

    fn needs_label(&self) -> &'static str {
        match self.needs_met_level {
            1 => "physiological",
            2 => "safety",
            3 => "belonging",
            4 => "esteem",
            5 => "self-actualize",
            _ => "unknown",
        }
    }
}

#[derive(Debug)]
struct BehaviorOutput {
    effective_empathy: f32,
    likely_to_help: bool,
    likely_truthful: bool,
    /// Degree of mutual reliance and emotional connection.
    /// Does NOT collapse to zero when helping is temporarily unavailable.
    interdependence: f32,
}

// ─── Display helpers ───────────────────────────────────────────

fn bar(value: f32, width: usize, max: f32) -> String {
    let filled = ((value / max) * width as f32).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!("{}{}", "#".repeat(filled), "-".repeat(empty))
}

// ─── Main ──────────────────────────────────────────────────────

fn main() {
    println!();
    println!("  PROSOCIAL PERSONALITY SIMULATOR");
    println!("  Inverse Dark Triad + Security Hypothesis Model");
    println!();

    let mut rng: StdRng = rand::make_rng();
    let people: Vec<ProsocialProfile> = (0..10)
        .map(|_| ProsocialProfile::simulate(&mut rng))
        .collect();

    // Header
    println!(
        "  {:4}  {:17}  {:9}  {:11}  {:14}  {:6}  {:4}  {:4}  {:6}",
        "ID", "Empathy", "Altruism", "Honesty", "Needs Level", "EffEmp", "Help", "True", "Inter"
    );
    println!("  {}", "-".repeat(90));

    for (i, p) in people.iter().enumerate() {
        let out = p.behavior_output();
        println!(
            "  P{:02}  [{}] {:5.1}%   {:>9}   {:>11}   {:>14}  {:5.1}%   {:4}   {:4}   {:5.1}",
            i + 1,
            bar(p.empathy, 5, 100.0),
            p.empathy,
            if p.altruistic {
                "selfless"
            } else {
                "conditional"
            },
            if p.honest_by_default {
                "default-true"
            } else {
                "situational"
            },
            p.needs_label(),
            out.effective_empathy,
            if out.likely_to_help { "yes" } else { "no" },
            if out.likely_truthful { "yes" } else { "no" },
            out.interdependence,
        );
    }

    // Aggregate statistics
    let outputs: Vec<BehaviorOutput> = people.iter().map(|p| p.behavior_output()).collect();

    let avg_empathy = people.iter().map(|p| p.empathy).sum::<f32>() / people.len() as f32;
    let avg_eff = outputs.iter().map(|o| o.effective_empathy).sum::<f32>() / people.len() as f32;
    let avg_inter = outputs.iter().map(|o| o.interdependence).sum::<f32>() / people.len() as f32;
    let altruist_count = people.iter().filter(|p| p.altruistic).count();
    let honest_count = people.iter().filter(|p| p.honest_by_default).count();
    let helpers = outputs.iter().filter(|o| o.likely_to_help).count();

    // Structural note: count people who can't help but are still embedded
    let connected_but_stretched = people
        .iter()
        .zip(outputs.iter())
        .filter(|(_, o)| !o.likely_to_help && o.interdependence > 40.0)
        .count();

    println!("  {}", "=".repeat(90));
    println!("  Aggregate over {} people:", people.len());
    println!("    Raw empathy:              {avg_empathy:.1}%");
    println!("    Effective empathy:        {avg_eff:.1}%  (reduced by unmet needs)");
    println!("    Avg interdependence:      {avg_inter:.1}%");
    println!(
        "    Altruistic:               {altruist_count}/{} ({:.0}%)",
        people.len(),
        altruist_count as f32 / people.len() as f32 * 100.0
    );
    println!(
        "    Honest default:           {honest_count}/{} ({:.0}%)",
        people.len(),
        honest_count as f32 / people.len() as f32 * 100.0
    );
    println!(
        "    Likely to help right now: {helpers}/{} ({:.0}%)",
        people.len(),
        helpers as f32 / people.len() as f32 * 100.0
    );
    println!(
        "    Connected but overstretched: {connected_but_stretched}/{}  \
         (still bonded, temporarily unable to give)",
        people.len()
    );

    println!();
    println!("  ─── Model notes ───────────────────────────────────────");
    println!("  • Empathy is raw capacity. Effective empathy is what's");
    println!("    actually available after accounting for unmet needs.");
    println!("  • A person at physiological level (1) retains only ~33%");
    println!("    prosocial capacity regardless of how empathetic they are.");
    println!("  • High empathy can override situational dishonesty —");
    println!("    lying to protect someone is itself a prosocial act.");
    println!("  • Interdependence reflects relational structure, not just");
    println!("    current willingness to help. Bonds don't dissolve when");
    println!("    someone is stretched thin — they may just go quiet.");
    println!("  • 'Connected but overstretched' captures a real phenomenon:");
    println!("    people who care deeply but currently have nothing to give.");
    println!();
}
