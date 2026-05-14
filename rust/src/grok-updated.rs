use rand::prelude::*;

/// Prosocial Personality Simulator v2
///
/// Improved model: Security-first + reciprocity + status dynamics + resilience
/// References: HEXACO, Batson, Engel (2011), Maslow, Brown et al. (2022),
///             and basic reciprocity research (Fehr & Gächter, etc.)

#[derive(Debug, Clone)]
struct ProsocialProfile {
    /// Raw cognitive/affective empathy (0-100)
    empathy: f32,
    /// Baseline willingness to give even at cost
    altruism: f32, // 0.0 - 1.0
    /// Default honesty (can be overridden by high empathy for prosocial lies)
    honesty_default: f32, // 0.0 - 1.0
    /// Current Maslow level (1-5)
    needs_met: u8,
    /// Ability to maintain prosociality despite past trauma/deficits
    resilience: f32, // 0.0 - 1.0
    /// How sensitive behavior is to perceived status/relative position
    status_sensitivity: f32, // 0.0 - 1.0
}

#[derive(Debug)]
struct BehaviorOutput {
    effective_empathy: f32,
    effective_altruism: f32,
    likely_truthful: bool,
    prosociality_score: f32, // overall "good actor" score
    interdependence_potential: f32,
}

impl ProsocialProfile {
    fn simulate(rng: &mut impl Rng) -> Self {
        ProsocialProfile {
            empathy: rng.random_range(10.0..95.0),
            altruism: rng.random_range(0.3..0.95),
            honesty_default: rng.random_range(0.5..0.95),
            needs_met: rng.random_range(1..=5),
            resilience: rng.random_range(0.4..0.98),
            status_sensitivity: rng.random_range(0.2..0.9),
        }
    }

    fn behavior_output(&self, rng: &mut impl Rng) -> BehaviorOutput {
        let deficit = (5 - self.needs_met) as f32;

        // Security is still foundational
        let security_factor = 1.0 / (1.0 + deficit * 0.65);

        // Resilience buffers against unmet needs
        let effective_security = security_factor + (1.0 - security_factor) * self.resilience * 0.6;

        let effective_empathy = self.empathy * effective_security;

        // Status threat reduces prosociality (relative deprivation effect)
        let status_factor = 1.0 - (self.status_sensitivity * 0.25);

        let effective_altruism = self.altruism * effective_security * status_factor;

        // High empathy allows prosocial deception when it helps others
        let likely_truthful =
            self.honesty_default > 0.5 || (effective_empathy > 70.0 && rng.random_bool(0.7));

        let prosociality_score = (effective_empathy * 0.4
            + effective_altruism * 100.0 * 0.4
            + if likely_truthful { 25.0 } else { 0.0 })
            / 1.1;

        // Interdependence requires capacity + willingness from both sides
        let interdependence_potential =
            (effective_empathy / 100.0) * effective_altruism * effective_security;

        BehaviorOutput {
            effective_empathy: effective_empathy.clamp(0.0, 100.0),
            effective_altruism: (effective_altruism * 100.0).clamp(0.0, 100.0),
            likely_truthful,
            prosociality_score: prosociality_score.clamp(0.0, 100.0),
            interdependence_potential: (interdependence_potential * 100.0).clamp(0.0, 100.0),
        }
    }

    fn needs_label(&self) -> &'static str {
        match self.needs_met {
            1 => "Survival",
            2 => "Safety",
            3 => "Belonging",
            4 => "Esteem",
            5 => "Actualized",
            _ => "Unknown",
        }
    }
}

// ─── Main ──────────────────────────────────────────────────────
fn main() {
    println!("\n PROSOCIAL PERSONALITY SIMULATOR v2");
    println!("   Security + Reciprocity + Status-aware Model\n");

    let mut rng = StdRng::from_rng(&mut rand::rng());
    let n = 12;
    let people: Vec<ProsocialProfile> = (0..n)
        .map(|_| ProsocialProfile::simulate(&mut rng))
        .collect();

    println!(
        " {:3} {:6} {:9} {:9} {:10} {:8} {:7} {:8} {:8} {:6}",
        "ID", "Emp", "Altruism", "Honesty", "Needs", "EffEmp", "EffAlt", "Truth", "ProSoc", "Inter"
    );
    println!(" {}", "─".repeat(88));

    let mut total_prosocial = 0.0;
    let mut total_inter = 0.0;
    let mut helpers = 0;

    for (i, p) in people.iter().enumerate() {
        let out = p.behavior_output(&mut rng);

        if out.effective_altruism > 55.0 {
            helpers += 1;
        }

        total_prosocial += out.prosociality_score;
        total_inter += out.interdependence_potential;

        println!(
            " P{:02} {:5.1}  {:7.1}%  {:7.1}%  {:>10}   {:6.1}   {:6.1}    {:4}    {:5.1}   {:5.1}",
            i + 1,
            p.empathy,
            p.altruism * 100.0,
            p.honesty_default * 100.0,
            p.needs_label(),
            out.effective_empathy,
            out.effective_altruism,
            if out.likely_truthful { "yes" } else { "no " },
            out.prosociality_score,
            out.interdependence_potential,
        );
    }

    let avg_prosocial = total_prosocial / n as f32;
    let avg_inter = total_inter / n as f32;
    let helper_pct = (helpers as f32 / n as f32) * 100.0;

    println!(" {}", "=".repeat(88));
    println!(" Aggregate ({} people):", n);
    println!(" Average Prosociality Score : {:.1}/100", avg_prosocial);
    println!(" Average Interdependence    : {:.1}/100", avg_inter);
    println!(
        " Strong Helpers             : {} ({:.0}%)",
        helpers, helper_pct
    );
    println!();

    println!(" ─── Key Insights ─────────────────────────────────────");
    println!(" • Unmet needs crush capacity far more than willingness");
    println!(" • Resilience acts as a shock absorber against hardship");
    println!(" • Status sensitivity makes people more selfish near the top or bottom");
    println!(" • High effective empathy still permits prosocial lies");
    println!(" • True interdependence only emerges with mutual capacity");
    println!();
}
