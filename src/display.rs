//! Turn a logic [`View`] into the names of the lit LCD segments.

use crate::logic::{Digit, Side, View, game::FloorView};

const RAMPS: [&str; 4] = ["tl", "bl", "tr", "br"];

/// Segments a..g lit for each digit.
fn digit_segments(d: u8) -> &'static [char] {
    match d {
        0 => &['a', 'b', 'c', 'd', 'e', 'f'],
        1 => &['b', 'c'],
        2 => &['a', 'b', 'g', 'e', 'd'],
        3 => &['a', 'b', 'g', 'c', 'd'],
        4 => &['f', 'g', 'b', 'c'],
        5 => &['a', 'f', 'g', 'c', 'd'],
        6 => &['a', 'f', 'g', 'e', 'd', 'c'],
        7 => &['a', 'b', 'c'],
        8 => &['a', 'b', 'c', 'd', 'e', 'f', 'g'],
        _ => &['a', 'b', 'c', 'd', 'f', 'g'],
    }
}

pub fn lit_segments(v: &View) -> Vec<String> {
    let mut out = Vec::new();
    for (r, row) in v.eggs.iter().enumerate() {
        for (i, &e) in row.iter().enumerate() {
            if e {
                out.push(format!("egg_{}_{i}", RAMPS[r]));
            }
        }
    }
    if let Some(w) = v.wolf {
        out.push(
            if w.side() == Side::Left {
                "wolf_l"
            } else {
                "wolf_r"
            }
            .into(),
        );
        out.push(format!("arm_{}", RAMPS[w.index()]));
    }
    if v.hare {
        out.push("hare".into());
    }
    if let Some(b) = v.bell {
        out.push(format!("bell_{b}"));
    }
    floor(&mut out, &v.left, 'l');
    floor(&mut out, &v.right, 'r');
    for (i, &m) in v.misses.iter().enumerate() {
        if m {
            out.push(format!("miss_{i}"));
        }
    }
    if v.one {
        out.push("one_b".into());
        out.push("one_c".into());
    }
    for (i, d) in v.digits.iter().enumerate() {
        digit(&mut out, i, *d);
    }
    for (on, name) in [
        (v.colon, "colon"),
        (v.am, "am"),
        (v.pm, "pm"),
        (v.alarm_icon, "alarm"),
        (v.game_a, "game_a"),
        (v.game_b, "game_b"),
    ] {
        if on {
            out.push(name.into());
        }
    }
    out
}

fn floor(out: &mut Vec<String>, f: &FloorView, side: char) {
    if f.falling {
        out.push(format!("falling_{side}"));
    }
    if f.broken {
        out.push(format!("broken_{side}"));
    }
    if f.hatched {
        out.push(format!("hatched_{side}"));
    }
    if let Some(c) = f.chick {
        out.push(format!("chick_{side}_{c}"));
    }
}

fn digit(out: &mut Vec<String>, cell: usize, d: Digit) {
    if let Some(d) = d {
        for s in digit_segments(d) {
            out.push(format!("d{cell}_{s}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::Ramp;

    #[test]
    fn wolf_and_score() {
        let v = View {
            wolf: Some(Ramp::BottomRight),
            digits: [None, Some(4), Some(1)],
            ..Default::default()
        };
        let lit = lit_segments(&v);
        assert!(lit.contains(&"wolf_r".into()));
        assert!(lit.contains(&"arm_br".into()));
        assert!(lit.contains(&"d1_f".into()));
        assert!(lit.contains(&"d2_b".into()));
        assert!(!lit.iter().any(|s| s.starts_with("d0_")));
    }

    /// Every name `lit_segments` can produce must exist in the shipped layout.
    #[test]
    fn layout_has_all_segments() {
        let layout: serde_json::Value =
            serde_json::from_str(include_str!("../assets/layout.json")).unwrap();
        let segs = layout["segments"].as_object().unwrap();
        let mut v = View {
            eggs: [[true; 5]; 4],
            hare: true,
            bell: Some(1),
            misses: [true; 3],
            one: true,
            digits: [Some(8); 3],
            colon: true,
            am: true,
            pm: true,
            alarm_icon: true,
            game_a: true,
            game_b: true,
            ..Default::default()
        };
        let mut names = Vec::new();
        for r in Ramp::ALL {
            v.wolf = Some(r);
            names.extend(lit_segments(&v));
        }
        for side in [&mut v.left, &mut v.right] {
            *side = FloorView {
                falling: true,
                broken: true,
                hatched: true,
                chick: Some(2),
            };
        }
        names.extend(lit_segments(&v));
        for c in 0..2 {
            v.left.chick = Some(c);
            v.right.chick = Some(c);
            names.extend(lit_segments(&v));
        }
        v.bell = Some(0);
        names.extend(lit_segments(&v));
        for n in names {
            assert!(segs.contains_key(&n), "layout.json lacks {n}");
        }
    }
}
