//! Unicode diacritic normalization for extracted text.
//!
//! Many PDFs (especially TeX output) draw accented letters as a base glyph
//! plus a separate spacing-accent glyph, and ToUnicode CMaps frequently map
//! these to spacing modifier characters (˘ ˆ ˇ ¸ …) rather than precomposed
//! letters. This pass folds an accent that is adjacent to a base letter — even
//! across a stray space — into the precomposed character (e.g. `a ˘` → `ă`),
//! and recombines base + combining-mark sequences. ASCII-only text is returned
//! unchanged and cheaply.

/// Recombine adjacent base-letter + diacritic sequences into precomposed forms.
pub fn normalize_diacritics(s: &str) -> String {
    // Fast path: nothing to do for pure ASCII.
    if s.is_ascii() {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut removed = vec![false; n];
    let mut out: Vec<char> = chars.clone();

    for i in 0..n {
        let kind = match accent_of(chars[i]) {
            Some(k) => k,
            None => continue,
        };
        // Look for a base letter immediately before (skipping one space), then
        // after, choosing whichever yields a valid precomposed character.
        if let Some(j) = base_before(&chars, &removed, i)
            && let Some(p) = precompose(out[j], kind)
        {
            out[j] = p;
            removed[i] = true;
            // Drop a single separating space between base and accent.
            if i >= 1 && chars[i - 1] == ' ' && i - 1 > j {
                removed[i - 1] = true;
            }
            continue;
        }
        if let Some(j) = base_after(&chars, &removed, i)
            && let Some(p) = precompose(out[j], kind)
        {
            out[j] = p;
            removed[i] = true;
            if i + 1 < n && chars[i + 1] == ' ' && i + 1 < j {
                removed[i + 1] = true;
            }
        }
    }

    let mut result = String::with_capacity(s.len());
    for i in 0..n {
        if !removed[i] {
            result.push(out[i]);
        }
    }
    result
}

fn base_before(chars: &[char], removed: &[bool], i: usize) -> Option<usize> {
    let mut j = i;
    let mut skipped_space = false;
    while j > 0 {
        j -= 1;
        if removed[j] {
            return None;
        }
        let c = chars[j];
        if c == ' ' && !skipped_space {
            skipped_space = true;
            continue;
        }
        if c.is_alphabetic() {
            return Some(j);
        }
        return None;
    }
    None
}

fn base_after(chars: &[char], removed: &[bool], i: usize) -> Option<usize> {
    let mut j = i + 1;
    let mut skipped_space = false;
    while j < chars.len() {
        if removed[j] {
            return None;
        }
        let c = chars[j];
        if c == ' ' && !skipped_space {
            skipped_space = true;
            j += 1;
            continue;
        }
        if c.is_alphabetic() {
            return Some(j);
        }
        return None;
    }
    None
}

/// Merge accent-only glyph fragments (a lone diacritic positioned above a base
/// letter) into the correct base character — the case TeX produces by drawing
/// the accent as a separate, separately-positioned glyph. Operates in place on
/// a page's fragments; fragments that are consumed are removed.
pub fn merge_positional_accents(frags: &mut Vec<crate::TextBlock>) {
    let n = frags.len();
    if n < 2 {
        return;
    }
    let mut removed = vec![false; n];

    for i in 0..n {
        // Is fragment i a lone accent glyph?
        let t = frags[i].text.trim();
        let mut ch = t.chars();
        let (first, second) = (ch.next(), ch.next());
        let accent = match (first, second) {
            (Some(c), None) => match accent_of(c) {
                Some(a) => a,
                None => continue,
            },
            _ => continue,
        };
        let a = &frags[i];
        let a_cx = a.x + a.width / 2.0;
        let a_y = a.y;

        // Find the base fragment whose x-range covers the accent and whose
        // baseline sits just below it (the accent is drawn above the letter).
        let mut best: Option<(usize, f64)> = None;
        for (j, b) in frags.iter().enumerate() {
            if j == i || removed[j] {
                continue;
            }
            let bt = b.text.trim();
            if bt.is_empty() || bt.chars().all(|c| accent_of(c).is_some()) {
                continue; // skip other accent glyphs / empties
            }
            if a_cx < b.x - 1.0 || a_cx > b.x + b.width + 1.0 {
                continue; // accent not horizontally over this fragment
            }
            // Accent should be at/above the base baseline, within ~1 line.
            let dy = a_y - b.y;
            if dy < -b.font_size * 0.3 || dy > b.font_size * 1.0 {
                continue;
            }
            let score = dy.abs();
            if best.map(|(_, s)| score < s).unwrap_or(true) {
                best = Some((j, score));
            }
        }

        if let Some((j, _)) = best
            && try_apply_accent(&mut frags[j], a_cx, accent)
        {
            removed[i] = true;
        }
    }

    if removed.iter().any(|&r| r) {
        let mut k = 0;
        frags.retain(|_| {
            let keep = !removed[k];
            k += 1;
            keep
        });
    }
}

/// Replace the character of `base` nearest `accent_cx` with its precomposed
/// form, if one exists. Returns whether a replacement was made.
fn try_apply_accent(base: &mut crate::TextBlock, accent_cx: f64, accent: Accent) -> bool {
    let chars: Vec<char> = base.text.chars().collect();
    if chars.is_empty() {
        return false;
    }
    let per_char = (base.width / chars.len() as f64).max(0.1);
    let approx = ((accent_cx - base.x) / per_char).floor() as isize;
    // Try the estimated index first, then nearby indices.
    let mut order: Vec<isize> = vec![approx];
    for d in 1..=2 {
        order.push(approx - d);
        order.push(approx + d);
    }
    for idx in order {
        if idx < 0 || idx as usize >= chars.len() {
            continue;
        }
        let ui = idx as usize;
        if let Some(p) = precompose(chars[ui], accent) {
            let mut new_chars = chars.clone();
            new_chars[ui] = p;
            base.text = new_chars.into_iter().collect();
            return true;
        }
    }
    false
}

#[derive(Clone, Copy, PartialEq)]
enum Accent {
    Acute,
    Grave,
    Circ,
    Tilde,
    Diaer,
    Caron,
    Breve,
    Ring,
    Macron,
    DotAbove,
    Cedilla,
    Ogonek,
    DoubleAcute,
}

fn accent_of(c: char) -> Option<Accent> {
    use Accent::*;
    Some(match c {
        '\u{00B4}' | '\u{0301}' | '\u{02CA}' => Acute,
        '`' | '\u{0300}' | '\u{02CB}' => Grave,
        '^' | '\u{02C6}' | '\u{0302}' => Circ,
        '~' | '\u{02DC}' | '\u{0303}' => Tilde,
        '\u{00A8}' | '\u{0308}' => Diaer,
        '\u{02C7}' | '\u{030C}' => Caron,
        '\u{02D8}' | '\u{0306}' => Breve,
        '\u{02DA}' | '\u{030A}' => Ring,
        '\u{00AF}' | '\u{02C9}' | '\u{0304}' => Macron,
        '\u{02D9}' | '\u{0307}' => DotAbove,
        '\u{00B8}' | '\u{0327}' => Cedilla,
        '\u{02DB}' | '\u{0328}' => Ogonek,
        '\u{02DD}' | '\u{030B}' => DoubleAcute,
        _ => return None,
    })
}

/// Map a base letter + accent to a precomposed character, when one exists.
fn precompose(base: char, a: Accent) -> Option<char> {
    use Accent::*;
    let c = match (base, a) {
        // a
        ('a', Acute) => 'á',
        ('a', Grave) => 'à',
        ('a', Circ) => 'â',
        ('a', Tilde) => 'ã',
        ('a', Diaer) => 'ä',
        ('a', Ring) => 'å',
        ('a', Breve) => 'ă',
        ('a', Macron) => 'ā',
        ('a', Ogonek) => 'ą',
        ('A', Acute) => 'Á',
        ('A', Grave) => 'À',
        ('A', Circ) => 'Â',
        ('A', Tilde) => 'Ã',
        ('A', Diaer) => 'Ä',
        ('A', Ring) => 'Å',
        ('A', Breve) => 'Ă',
        ('A', Macron) => 'Ā',
        ('A', Ogonek) => 'Ą',
        // e
        ('e', Acute) => 'é',
        ('e', Grave) => 'è',
        ('e', Circ) => 'ê',
        ('e', Diaer) => 'ë',
        ('e', Caron) => 'ě',
        ('e', Macron) => 'ē',
        ('e', Ogonek) => 'ę',
        ('e', DotAbove) => 'ė',
        ('e', Breve) => 'ĕ',
        ('E', Acute) => 'É',
        ('E', Grave) => 'È',
        ('E', Circ) => 'Ê',
        ('E', Diaer) => 'Ë',
        ('E', Caron) => 'Ě',
        ('E', Macron) => 'Ē',
        ('E', Ogonek) => 'Ę',
        // i
        ('i', Acute) => 'í',
        ('i', Grave) => 'ì',
        ('i', Circ) => 'î',
        ('i', Diaer) => 'ï',
        ('i', Tilde) => 'ĩ',
        ('i', Macron) => 'ī',
        ('I', Acute) => 'Í',
        ('I', Grave) => 'Ì',
        ('I', Circ) => 'Î',
        ('I', Diaer) => 'Ï',
        ('I', Tilde) => 'Ĩ',
        ('I', Macron) => 'Ī',
        // o
        ('o', Acute) => 'ó',
        ('o', Grave) => 'ò',
        ('o', Circ) => 'ô',
        ('o', Tilde) => 'õ',
        ('o', Diaer) => 'ö',
        ('o', Macron) => 'ō',
        ('o', DoubleAcute) => 'ő',
        ('O', Acute) => 'Ó',
        ('O', Grave) => 'Ò',
        ('O', Circ) => 'Ô',
        ('O', Tilde) => 'Õ',
        ('O', Diaer) => 'Ö',
        ('O', Macron) => 'Ō',
        ('O', DoubleAcute) => 'Ő',
        // u
        ('u', Acute) => 'ú',
        ('u', Grave) => 'ù',
        ('u', Circ) => 'û',
        ('u', Diaer) => 'ü',
        ('u', Ring) => 'ů',
        ('u', Tilde) => 'ũ',
        ('u', Macron) => 'ū',
        ('u', DoubleAcute) => 'ű',
        ('u', Breve) => 'ŭ',
        ('U', Acute) => 'Ú',
        ('U', Grave) => 'Ù',
        ('U', Circ) => 'Û',
        ('U', Diaer) => 'Ü',
        ('U', Ring) => 'Ů',
        ('U', Macron) => 'Ū',
        // y
        ('y', Acute) => 'ý',
        ('y', Diaer) => 'ÿ',
        ('Y', Acute) => 'Ý',
        // consonants
        ('c', Acute) => 'ć',
        ('c', Caron) => 'č',
        ('c', Cedilla) => 'ç',
        ('c', Circ) => 'ĉ',
        ('c', DotAbove) => 'ċ',
        ('C', Acute) => 'Ć',
        ('C', Caron) => 'Č',
        ('C', Cedilla) => 'Ç',
        ('n', Tilde) => 'ñ',
        ('n', Acute) => 'ń',
        ('n', Caron) => 'ň',
        ('N', Tilde) => 'Ñ',
        ('N', Acute) => 'Ń',
        ('N', Caron) => 'Ň',
        ('s', Acute) => 'ś',
        ('s', Caron) => 'š',
        ('s', Cedilla) => 'ş',
        ('S', Acute) => 'Ś',
        ('S', Caron) => 'Š',
        ('S', Cedilla) => 'Ş',
        ('z', Acute) => 'ź',
        ('z', Caron) => 'ž',
        ('z', DotAbove) => 'ż',
        ('Z', Acute) => 'Ź',
        ('Z', Caron) => 'Ž',
        ('Z', DotAbove) => 'Ż',
        ('t', Caron) => 'ť',
        ('t', Cedilla) => 'ţ',
        ('T', Caron) => 'Ť',
        ('T', Cedilla) => 'Ţ',
        ('r', Caron) => 'ř',
        ('r', Acute) => 'ŕ',
        ('R', Caron) => 'Ř',
        ('R', Acute) => 'Ŕ',
        ('d', Caron) => 'ď',
        ('D', Caron) => 'Ď',
        ('l', Acute) => 'ĺ',
        ('l', Caron) => 'ľ',
        ('L', Acute) => 'Ĺ',
        ('L', Caron) => 'Ľ',
        ('g', Breve) => 'ğ',
        ('g', Cedilla) => 'ģ',
        ('g', DotAbove) => 'ġ',
        ('G', Breve) => 'Ğ',
        ('w', Circ) => 'ŵ',
        ('w', Acute) => 'ẃ',
        ('h', Circ) => 'ĥ',
        _ => return None,
    };
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_unchanged() {
        assert_eq!(normalize_diacritics("Hello, world!"), "Hello, world!");
    }

    #[test]
    fn combines_spacing_breve() {
        // "sa ˘ na ˘ tos" → spacing breve after vowels → "sănătos"
        assert_eq!(normalize_diacritics("sa\u{02D8}na\u{02D8}tos"), "sănătos");
    }

    #[test]
    fn combines_with_space() {
        assert_eq!(normalize_diacritics("a \u{02D8}"), "ă");
        assert_eq!(normalize_diacritics("Cafe\u{0301}"), "Café");
    }

    #[test]
    fn combines_following_base() {
        // accent before base (circumflex then i) → î
        assert_eq!(normalize_diacritics("\u{02C6}i"), "î");
    }

    fn tb(text: &str, x: f64, y: f64, w: f64, size: f64) -> crate::TextBlock {
        crate::TextBlock {
            text: text.into(),
            x,
            y,
            width: w,
            height: size,
            font_size: size,
            font_name: "F".into(),
            page_number: 1,
        }
    }

    #[test]
    fn positional_accent_merges_into_base() {
        // Base "atos" at x=100 width 40 (10/char); breve accent over the first
        // 'a' (centered near x=105), drawn slightly above the baseline.
        let mut frags = vec![
            tb("atos", 100.0, 500.0, 40.0, 10.0),
            tb("\u{02D8}", 103.0, 504.0, 4.0, 7.0),
        ];
        merge_positional_accents(&mut frags);
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0].text, "ătos");
    }

    #[test]
    fn positional_accent_picks_correct_char() {
        // Accent over the 'a' (index 1) of "natos" → "nătos".
        let mut frags = vec![
            tb("natos", 100.0, 500.0, 50.0, 10.0),  // 10 pts/char
            tb("\u{02D8}", 119.0, 505.0, 4.0, 7.0), // center ~121, snaps to 'a'
        ];
        merge_positional_accents(&mut frags);
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0].text, "nătos");
    }
}
