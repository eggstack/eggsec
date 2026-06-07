use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvssScore {
    pub base_score: f32,
    pub temporal_score: f32,
    pub environmental_score: f32,
    pub vector: String,
}

impl CvssScore {
    pub fn from_vector(vector: &str) -> Result<Self> {
        let parsed = parse_vector(vector)?;
        let base_score = compute_base_score(&parsed);

        let temporal_score = compute_temporal_score(base_score, &parsed);
        let environmental_score = compute_environmental_score(&parsed);

        Ok(Self {
            base_score,
            temporal_score,
            environmental_score,
            vector: vector.to_string(),
        })
    }

    pub fn base_score(&self) -> f32 {
        self.base_score
    }

    pub fn severity(&self) -> &'static str {
        if self.base_score <= 0.0 {
            "NONE"
        } else if self.base_score <= 3.9 {
            "LOW"
        } else if self.base_score <= 6.9 {
            "MEDIUM"
        } else if self.base_score <= 8.9 {
            "HIGH"
        } else {
            "CRITICAL"
        }
    }

    pub fn temporal_score(&self) -> f32 {
        self.temporal_score
    }

    pub fn environmental_score(&self) -> f32 {
        self.environmental_score
    }

    #[allow(clippy::too_many_arguments)]
    pub fn calculate_base(
        attack_vector: &str,
        attack_complexity: &str,
        privileges_required: &str,
        user_interaction: &str,
        scope: &str,
        confidentiality: &str,
        integrity: &str,
        availability: &str,
    ) -> f32 {
        let av = av_weight(attack_vector);
        let ac = ac_weight(attack_complexity);
        let pr_u = pr_weight_unchanged(privileges_required);
        let pr_c = pr_weight_changed(privileges_required);
        let ui = ui_weight(user_interaction);
        let c = cia_weight(confidentiality);
        let i = cia_weight(integrity);
        let a = cia_weight(availability);

        let iss = 1.0 - ((1.0 - c) * (1.0 - i) * (1.0 - a));

        if scope == "U" {
            let impact = 6.42 * iss;
            let exploitability = 8.22 * av * ac * pr_u * ui;
            if impact == 0.0 {
                0.0
            } else {
                let score = (impact + exploitability).min(10.0);
                f32::floor(score * 10.0) / 10.0
            }
        } else {
            let impact = 7.52 * (iss - 0.029) - 3.25 * (iss - 0.02).powf(15.0);
            let exploitability = 8.22 * av * ac * pr_c * ui;
            if impact == 0.0 {
                0.0
            } else {
                let score = (1.08 * (impact + exploitability)).min(10.0);
                f32::floor(score * 10.0) / 10.0
            }
        }
    }
}

fn av_weight(av: &str) -> f32 {
    match av {
        "N" => 0.85,
        "A" => 0.62,
        "L" => 0.55,
        "P" => 0.20,
        _ => 0.85,
    }
}

fn ac_weight(ac: &str) -> f32 {
    match ac {
        "L" => 0.77,
        "H" => 0.44,
        _ => 0.77,
    }
}

fn pr_weight_unchanged(pr: &str) -> f32 {
    match pr {
        "N" => 0.85,
        "L" => 0.62,
        "H" => 0.27,
        _ => 0.85,
    }
}

fn pr_weight_changed(pr: &str) -> f32 {
    match pr {
        "N" => 0.85,
        "L" => 0.68,
        "H" => 0.50,
        _ => 0.85,
    }
}

fn ui_weight(ui: &str) -> f32 {
    match ui {
        "N" => 0.85,
        "R" => 0.62,
        _ => 0.85,
    }
}

fn cia_weight(val: &str) -> f32 {
    match val {
        "H" => 0.56,
        "L" => 0.22,
        "N" => 0.0,
        _ => 0.0,
    }
}

fn cia_requirement_weight(val: &str) -> f32 {
    match val {
        "H" => 1.5,
        "M" => 1.0,
        "L" => 0.5,
        _ => 1.0,
    }
}

fn temporal_metric_weight(metric: &str) -> f32 {
    match metric {
        "H" => 0.91,
        "M" => 0.94,
        "L" => 0.97,
        "U" => 0.95,
        "W" => 0.96,
        "O" => 0.98,
        "C" => 1.0,
        "R" => 0.96,
        _ => 1.0,
    }
}

struct ParsedVector {
    av: String,
    ac: String,
    pr: String,
    ui: String,
    scope: String,
    c: String,
    i: String,
    a: String,
    e: String,
    rl: String,
    rc: String,
    cr: String,
    ir: String,
    ar: String,
    mav: String,
    mac: String,
    mpr: String,
    mui: String,
    ms: String,
    mc: String,
    mi: String,
    ma: String,
}

fn parse_vector(vector: &str) -> Result<ParsedVector> {
    let mut v = ParsedVector {
        av: "N".to_string(),
        ac: "L".to_string(),
        pr: "N".to_string(),
        ui: "N".to_string(),
        scope: "U".to_string(),
        c: "N".to_string(),
        i: "N".to_string(),
        a: "N".to_string(),
        e: "X".to_string(),
        rl: "X".to_string(),
        rc: "X".to_string(),
        cr: "X".to_string(),
        ir: "X".to_string(),
        ar: "X".to_string(),
        mav: "X".to_string(),
        mac: "X".to_string(),
        mpr: "X".to_string(),
        mui: "X".to_string(),
        ms: "X".to_string(),
        mc: "X".to_string(),
        mi: "X".to_string(),
        ma: "X".to_string(),
    };

    for part in vector.split('/') {
        let parts: Vec<&str> = part.split(':').collect();
        if parts.len() != 2 {
            continue;
        }
        let key = parts[0];
        let val = parts[1];
        match key {
            "AV" => v.av = val.to_string(),
            "AC" => v.ac = val.to_string(),
            "PR" => v.pr = val.to_string(),
            "UI" => v.ui = val.to_string(),
            "S" => v.scope = val.to_string(),
            "C" => v.c = val.to_string(),
            "I" => v.i = val.to_string(),
            "A" => v.a = val.to_string(),
            "E" => v.e = val.to_string(),
            "RL" => v.rl = val.to_string(),
            "RC" => v.rc = val.to_string(),
            "CR" => v.cr = val.to_string(),
            "IR" => v.ir = val.to_string(),
            "AR" => v.ar = val.to_string(),
            "MAV" => v.mav = val.to_string(),
            "MAC" => v.mac = val.to_string(),
            "MPR" => v.mpr = val.to_string(),
            "MUI" => v.mui = val.to_string(),
            "MS" => v.ms = val.to_string(),
            "MC" => v.mc = val.to_string(),
            "MI" => v.mi = val.to_string(),
            "MA" => v.ma = val.to_string(),
            _ => {}
        }
    }

    Ok(v)
}

fn compute_base_score(v: &ParsedVector) -> f32 {
    let av = av_weight(&v.av);
    let ac = ac_weight(&v.ac);
    let pr_u = pr_weight_unchanged(&v.pr);
    let pr_c = pr_weight_changed(&v.pr);
    let ui = ui_weight(&v.ui);
    let c = cia_weight(&v.c);
    let i = cia_weight(&v.i);
    let a = cia_weight(&v.a);

    let iss = 1.0 - ((1.0 - c) * (1.0 - i) * (1.0 - a));

    if v.scope == "U" {
        let impact = 6.42 * iss;
        let exploitability = 8.22 * av * ac * pr_u * ui;
        if impact == 0.0 {
            0.0
        } else {
            let score = (impact + exploitability).min(10.0);
            f32::floor(score * 10.0) / 10.0
        }
    } else {
        let impact = 7.52 * (iss - 0.029) - 3.25 * (iss - 0.02).powf(15.0);
        let exploitability = 8.22 * av * ac * pr_c * ui;
        if impact == 0.0 {
            0.0
        } else {
            let score = (1.08 * (impact + exploitability)).min(10.0);
            f32::floor(score * 10.0) / 10.0
        }
    }
}

fn compute_temporal_score(base_score: f32, v: &ParsedVector) -> f32 {
    let e = temporal_metric_weight(&v.e);
    let rl = temporal_metric_weight(&v.rl);
    let rc = temporal_metric_weight(&v.rc);

    let score = base_score * e * rl * rc;
    let score = score.min(10.0);
    f32::floor(score * 10.0) / 10.0
}

fn compute_environmental_score(v: &ParsedVector) -> f32 {
    let cr = cia_requirement_weight(&v.cr);
    let ir = cia_requirement_weight(&v.ir);
    let ar = cia_requirement_weight(&v.ar);

    let c_modified = modified_cia(&v.c, &v.mc, cr);
    let i_modified = modified_cia(&v.i, &v.mi, ir);
    let a_modified = modified_cia(&v.a, &v.ma, ar);

    let av = av_weight(if v.mav == "X" { &v.av } else { &v.mav });
    let ac = ac_weight(if v.mac == "X" { &v.ac } else { &v.mac });
    let scope = if v.ms == "X" { &v.scope } else { &v.ms };
    let pr = if v.mpr == "X" { &v.pr } else { &v.mpr };
    let ui = ui_weight(if v.mui == "X" { &v.ui } else { &v.mui });

    let pr_u = pr_weight_unchanged(pr);
    let pr_c = pr_weight_changed(pr);

    let iss = 1.0 - ((1.0 - c_modified) * (1.0 - i_modified) * (1.0 - a_modified));

    let (impact, exploitability) = if scope == "U" {
        (6.42 * iss, 8.22 * av * ac * pr_u * ui)
    } else {
        let impact = 7.52 * (iss - 0.029) - 3.25 * (iss - 0.02).powf(15.0);
        (impact, 8.22 * av * ac * pr_c * ui)
    };

    if impact == 0.0 {
        return 0.0;
    }

    let score = if scope == "U" {
        (impact + exploitability).min(10.0)
    } else {
        (1.08 * (impact + exploitability)).min(10.0)
    };

    let e = temporal_metric_weight(&v.e);
    let rl = temporal_metric_weight(&v.rl);
    let rc = temporal_metric_weight(&v.rc);

    let score = (score * e * rl * rc).min(10.0);
    f32::floor(score * 10.0) / 10.0
}

fn modified_cia(base: &str, modified: &str, requirement_weight: f32) -> f32 {
    let val = if modified == "X" { base } else { modified };
    let weight = cia_weight(val);
    (weight * requirement_weight).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cvss_score_from_vector() {
        let score =
            CvssScore::from_vector("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H").unwrap();
        assert!(score.base_score >= 9.0);
    }

    #[test]
    fn test_base_score_calculation() {
        let score = CvssScore::calculate_base("N", "L", "N", "N", "U", "H", "H", "H");
        assert!(score >= 9.0);
    }

    #[test]
    fn test_temporal_score_applied() {
        let with_exploit =
            CvssScore::from_vector("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H/E:H/RL:U/RC:C")
                .unwrap();
        let without_exploit =
            CvssScore::from_vector("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H").unwrap();
        assert!(with_exploit.temporal_score <= without_exploit.temporal_score);
    }

    #[test]
    fn test_environmental_score_with_requirements() {
        let high_req = CvssScore::from_vector(
            "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H/CR:H/IR:H/AR:H",
        )
        .unwrap();
        let low_req = CvssScore::from_vector(
            "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H/CR:L/IR:L/AR:L",
        )
        .unwrap();
        assert!(high_req.environmental_score >= low_req.environmental_score);
    }

    #[test]
    fn test_environmental_with_modified_metrics() {
        let modified = CvssScore::from_vector(
            "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H/MAV:A/MAC:H/MPR:L",
        )
        .unwrap();
        assert!(modified.environmental_score < modified.base_score);
    }

    #[test]
    fn test_scope_changed_pr_weight() {
        let scope_u =
            CvssScore::from_vector("CVSS:3.1/AV:N/AC:L/PR:H/UI:N/S:U/C:H/I:H/A:H").unwrap();
        let scope_c =
            CvssScore::from_vector("CVSS:3.1/AV:N/AC:L/PR:H/UI:N/S:C/C:H/I:H/A:H").unwrap();
        assert!(scope_c.base_score > scope_u.base_score);
    }

    #[test]
    fn test_severity_classification() {
        let critical =
            CvssScore::from_vector("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H").unwrap();
        assert_eq!(critical.severity(), "CRITICAL");

        let low =
            CvssScore::from_vector("CVSS:3.1/AV:L/AC:H/PR:H/UI:R/S:U/C:L/I:L/A:N").unwrap();
        assert_eq!(low.severity(), "LOW");
    }
}
