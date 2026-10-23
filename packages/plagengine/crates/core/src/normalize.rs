//! Stage 1 — normalization and tokenization.
//!
//! Every original char is folded independently: NFKD → drop combining marks →
//! homoglyph fold to Latin → lowercase. Invisible chars (zero-width, bidi
//! controls, …) are dropped without breaking a token, so `pla\u{200B}giarism`
//! still yields `plagiarism`. Hidden spans (e.g. white text in a .docx) act as
//! token separators and contribute no tokens. Each token keeps the original
//! char range it was built from.

use crate::types::{merge_spans, Span};
use serde::{Deserialize, Serialize};
use unicode_normalization::char::{decompose_canonical, decompose_compatible, is_combining_mark};
use xxhash_rust::xxh3::xxh3_64;

/// One word of the normalized stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    /// Hash of the normalized token text.
    pub hash: u64,
    /// Original char range `[start, end)`.
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub fn span(&self) -> Span {
        Span::new(self.start, self.end)
    }
}

/// A char that was mapped from another script / compatibility form onto a
/// Latin letter inside Latin text — Turnitin's "replaced characters".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplacedChar {
    pub pos: usize,
    pub original: char,
    pub replacement: char,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvisibleKind {
    ZeroWidth,
    BidiControl,
    SoftHyphen,
    TagCharacter,
    VariationSelector,
    Filler,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvisibleChar {
    pub pos: usize,
    pub ch: char,
    pub kind: InvisibleKind,
}

/// Result of normalizing one document.
#[derive(Clone, Debug, Default)]
pub struct NormDoc {
    /// Original chars (indices used by every span).
    pub chars: Vec<char>,
    pub tokens: Vec<Token>,
    pub replaced: Vec<ReplacedChar>,
    pub invisible: Vec<InvisibleChar>,
    /// Sorted, merged hidden spans used during normalization.
    pub hidden: Vec<Span>,
}

impl NormDoc {
    pub fn token_hashes(&self) -> Vec<u64> {
        self.tokens.iter().map(|t| t.hash).collect()
    }

    /// Index of the first token whose start is `>= pos`.
    pub fn token_at_or_after(&self, pos: usize) -> usize {
        self.tokens.partition_point(|t| t.start < pos)
    }

    /// Number of tokens that start inside `span`.
    pub fn tokens_in(&self, span: Span) -> usize {
        self.token_at_or_after(span.end) - self.token_at_or_after(span.start)
    }

    /// Prefix sums of countable chars: non-whitespace, not hidden, not
    /// invisible. `prefix[i]` = countable chars in `[0, i)`.
    pub fn countable_prefix(&self) -> Vec<u32> {
        let mut excluded = vec![false; self.chars.len()];
        for h in &self.hidden {
            for e in excluded.iter_mut().take(h.end.min(self.chars.len())).skip(h.start) {
                *e = true;
            }
        }
        for inv in &self.invisible {
            excluded[inv.pos] = true;
        }
        let mut prefix = Vec::with_capacity(self.chars.len() + 1);
        let mut acc = 0u32;
        prefix.push(0);
        for (i, c) in self.chars.iter().enumerate() {
            if !excluded[i] && !c.is_whitespace() && is_invisible(*c).is_none() {
                acc += 1;
            }
            prefix.push(acc);
        }
        prefix
    }
}

/// Classify a char as invisible/format-control, if it is one.
pub fn is_invisible(c: char) -> Option<InvisibleKind> {
    use InvisibleKind::*;
    Some(match c {
        '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{2061}'..='\u{2064}' | '\u{FEFF}' | '\u{180E}' | '\u{034F}' => ZeroWidth,
        '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{061C}' => BidiControl,
        '\u{00AD}' => SoftHyphen,
        '\u{E0000}'..='\u{E007F}' => TagCharacter,
        '\u{FE00}'..='\u{FE0F}' | '\u{E0100}'..='\u{E01EF}' => VariationSelector,
        '\u{115F}' | '\u{1160}' | '\u{3164}' | '\u{FFA0}' => Filler,
        _ => return None,
    })
}

/// Map look-alike letters from other scripts (and Latin small capitals etc.)
/// to the Latin letter they imitate. Output is lowercase ASCII.
pub fn homoglyph(c: char) -> Option<char> {
    Some(match c {
        // Cyrillic lowercase
        'а' => 'a', 'в' => 'b', 'е' => 'e', 'һ' => 'h', 'і' => 'i', 'ј' => 'j', 'к' => 'k',
        'ӏ' => 'l', 'м' => 'm', 'н' => 'h', 'о' => 'o', 'р' => 'p', 'ԛ' => 'q', 'ѕ' => 's',
        'т' => 't', 'с' => 'c', 'у' => 'y', 'ԝ' => 'w', 'х' => 'x', 'ԁ' => 'd', 'ү' => 'y',
        'ɡ' => 'g', 'ı' => 'i', 'ȷ' => 'j',
        // Cyrillic uppercase
        'А' => 'a', 'В' => 'b', 'Е' => 'e', 'Һ' => 'h', 'І' => 'i', 'Ј' => 'j', 'К' => 'k',
        'Ӏ' => 'l', 'М' => 'm', 'Н' => 'h', 'О' => 'o', 'Р' => 'p', 'Ԛ' => 'q', 'Ѕ' => 's',
        'Т' => 't', 'С' => 'c', 'У' => 'y', 'Ԝ' => 'w', 'Х' => 'x', 'Ԁ' => 'd', 'Ү' => 'y',
        // Greek
        'α' => 'a', 'ε' => 'e', 'ι' => 'i', 'κ' => 'k', 'ν' => 'v', 'ο' => 'o', 'ρ' => 'p',
        'τ' => 't', 'υ' => 'u', 'χ' => 'x', 'ω' => 'w', 'γ' => 'y',
        'Α' => 'a', 'Β' => 'b', 'Ε' => 'e', 'Ζ' => 'z', 'Η' => 'h', 'Ι' => 'i', 'Κ' => 'k',
        'Μ' => 'm', 'Ν' => 'n', 'Ο' => 'o', 'Ρ' => 'p', 'Τ' => 't', 'Υ' => 'y', 'Χ' => 'x',
        // Armenian
        'օ' => 'o', 'ս' => 'u', 'հ' => 'h', 'ո' => 'n', 'ց' => 'g', 'զ' => 'q',
        // Cherokee (uppercase forms look like Latin capitals)
        'Ꭺ' => 'a', 'Ᏼ' => 'b', 'Ꮯ' => 'c', 'Ꭰ' => 'd', 'Ꭼ' => 'e', 'Ꮐ' => 'g', 'Ꮋ' => 'h',
        'Ꭻ' => 'j', 'Ꮶ' => 'k', 'Ꮮ' => 'l', 'Ꮇ' => 'm', 'Ꮲ' => 'p', 'Ꮢ' => 'r', 'Ꮪ' => 's',
        'Ꮩ' => 'v', 'Ꮃ' => 'w', 'Ꮓ' => 'z',
        // Latin small capitals & friends
        'ᴀ' => 'a', 'ʙ' => 'b', 'ᴄ' => 'c', 'ᴅ' => 'd', 'ᴇ' => 'e', 'ɢ' => 'g', 'ʜ' => 'h',
        'ɪ' => 'i', 'ᴊ' => 'j', 'ᴋ' => 'k', 'ʟ' => 'l', 'ᴍ' => 'm', 'ɴ' => 'n', 'ᴏ' => 'o',
        'ᴘ' => 'p', 'ʀ' => 'r', 'ꜱ' => 's', 'ᴛ' => 't', 'ᴜ' => 'u', 'ᴠ' => 'v', 'ᴡ' => 'w',
        'ʏ' => 'y', 'ᴢ' => 'z',
        _ => return None,
    })
}

/// Is `c` a Latin ligature (legitimately produced by PDF extraction)?
fn is_ligature(c: char) -> bool {
    matches!(c, '\u{FB00}'..='\u{FB06}' | 'Ĳ' | 'ĳ' | 'Œ' | 'œ' | 'Æ' | 'æ')
}

/// If `c` imitates a Latin letter (homoglyph from another script, or a
/// compatibility form such as fullwidth/mathematical letters), return the
/// ASCII letter it stands for. Accented Latin letters and ligatures return
/// `None`.
pub fn confusable(c: char) -> Option<char> {
    if c.is_ascii() || !c.is_alphabetic() || is_ligature(c) {
        return None;
    }
    if let Some(h) = homoglyph(c) {
        return Some(h);
    }
    // Accented Latin letters decompose canonically to an ASCII base: legit.
    let mut canon_first = None;
    decompose_canonical(c, |d| {
        canon_first.get_or_insert(d);
    });
    if canon_first.is_some_and(|d| d.is_ascii_alphabetic()) {
        return None;
    }
    // Compatibility forms (fullwidth, mathematical alphanumerics, …).
    let mut out = None;
    let mut n = 0;
    decompose_compatible(c, |d| {
        if !is_combining_mark(d) {
            n += 1;
            out = Some(d);
        }
    });
    match out {
        Some(d) if n == 1 && d.is_ascii_alphabetic() => Some(d.to_ascii_lowercase()),
        _ => None,
    }
}

fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{3040}'..='\u{30FF}' | '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' |
        '\u{F900}'..='\u{FAFF}' | '\u{AC00}'..='\u{D7AF}' | '\u{20000}'..='\u{2FA1F}')
}

/// Fold one original char into its normalized form, pushing onto `out`.
#[inline]
pub fn fold_char(c: char, out: &mut String) {
    if c.is_ascii() {
        out.push(c.to_ascii_lowercase());
        return;
    }
    if let Some(h) = homoglyph(c) {
        out.push(h);
        return;
    }
    decompose_compatible(c, |d| {
        if is_combining_mark(d) {
            return;
        }
        if let Some(h) = homoglyph(d) {
            out.push(h);
        } else {
            out.extend(d.to_lowercase());
        }
    });
}

/// Normalize and tokenize `text`. `hidden` are char spans to treat as
/// separators (they produce no tokens).
pub fn normalize(text: &str, hidden: &[Span]) -> NormDoc {
    let chars: Vec<char> = text.chars().collect();
    let hidden = merge_spans(hidden.to_vec());
    let mut tokens = Vec::with_capacity(chars.len() / 5);
    let mut invisible = Vec::new();

    let mut tok_buf = String::new();
    let mut tok_start = 0usize;
    let mut tok_end = 0usize;
    let mut fold_buf = String::with_capacity(8);
    let mut hid_idx = 0usize;

    let flush = |buf: &mut String, start: usize, end: usize, tokens: &mut Vec<Token>| {
        if !buf.is_empty() {
            tokens.push(Token { hash: xxh3_64(buf.as_bytes()), start, end });
            buf.clear();
        }
    };

    for (i, &c) in chars.iter().enumerate() {
        while hid_idx < hidden.len() && hidden[hid_idx].end <= i {
            hid_idx += 1;
        }
        if hid_idx < hidden.len() && hidden[hid_idx].contains(i) {
            flush(&mut tok_buf, tok_start, tok_end, &mut tokens);
            continue;
        }
        if let Some(kind) = is_invisible(c) {
            if !(i == 0 && c == '\u{FEFF}') {
                invisible.push(InvisibleChar { pos: i, ch: c, kind });
            }
            continue;
        }
        fold_buf.clear();
        fold_char(c, &mut fold_buf);
        for f in fold_buf.chars() {
            if f.is_alphanumeric() {
                if is_cjk(f) {
                    flush(&mut tok_buf, tok_start, tok_end, &mut tokens);
                    tok_buf.push(f);
                    flush(&mut tok_buf, i, i + 1, &mut tokens);
                    continue;
                }
                if tok_buf.is_empty() {
                    tok_start = i;
                }
                tok_buf.push(f);
                tok_end = i + 1;
            } else {
                flush(&mut tok_buf, tok_start, tok_end, &mut tokens);
            }
        }
    }
    flush(&mut tok_buf, tok_start, tok_end, &mut tokens);

    let replaced = detect_replaced(&chars, &tokens);
    NormDoc { chars, tokens, replaced, invisible, hidden }
}

/// Find replaced characters: confusables inside mixed-script words, or
/// words made entirely of confusables (≥ 3 letters) in Latin-dominant text.
fn detect_replaced(chars: &[char], tokens: &[Token]) -> Vec<ReplacedChar> {
    let mut ascii_letters = 0usize;
    let mut other_letters = 0usize;
    let mut any_confusable = false;
    for &c in chars {
        if c.is_ascii_alphabetic() {
            ascii_letters += 1;
        } else if c.is_alphabetic() {
            other_letters += 1;
            any_confusable |= confusable(c).is_some();
        }
    }
    if !any_confusable {
        return Vec::new();
    }
    let latin_dominant = ascii_letters > other_letters;
    let mut out = Vec::new();
    for t in tokens {
        let word = &chars[t.start..t.end];
        let mut has_ascii = false;
        let mut n_conf = 0;
        let mut n_letters = 0;
        for &c in word {
            if c.is_alphabetic() {
                n_letters += 1;
                if c.is_ascii() {
                    has_ascii = true;
                } else if confusable(c).is_some() {
                    n_conf += 1;
                }
            }
        }
        if n_conf == 0 {
            continue;
        }
        let mixed = has_ascii;
        let all_conf = latin_dominant && n_conf == n_letters && n_letters >= 3;
        let mostly_latin = latin_dominant && word.iter().filter(|c| c.is_alphabetic()).all(|&c| c.is_ascii() || confusable(c).is_some());
        if (mixed && mostly_latin) || all_conf {
            for (off, &c) in word.iter().enumerate() {
                if let Some(r) = confusable(c) {
                    out.push(ReplacedChar { pos: t.start + off, original: c, replacement: r });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(n: &NormDoc) -> Vec<String> {
        n.tokens.iter().map(|t| n.chars[t.start..t.end].iter().collect()).collect()
    }

    fn hashes(s: &str) -> Vec<u64> {
        normalize(s, &[]).token_hashes()
    }

    #[test]
    fn tokenizes_with_offsets() {
        let n = normalize("Hello, wörld! 42x", &[]);
        assert_eq!(words(&n), vec!["Hello", "wörld", "42x"]);
        assert_eq!(n.tokens[1].span(), Span::new(7, 12));
    }

    #[test]
    fn case_accents_and_nfkc_fold() {
        assert_eq!(hashes("Café NAÏVE"), hashes("cafe naive"));
        assert_eq!(hashes("ﬁnal ｆｕｌｌｗｉｄｔｈ 𝐛𝐨𝐥𝐝"), hashes("final fullwidth bold"));
    }

    #[test]
    fn homoglyphs_fold_and_are_flagged() {
        // Cyrillic а, е, о, р, с inside English words.
        let text = "The quick brоwn fox jumps оvеr thе lazy dog; sсience is fun.";
        let n = normalize(text, &[]);
        assert_eq!(n.token_hashes(), hashes("The quick brown fox jumps over the lazy dog; science is fun."));
        let chars: Vec<char> = n.replaced.iter().map(|r| r.original).collect();
        assert_eq!(chars, vec!['о', 'о', 'е', 'е', 'с']);
        assert!(n.replaced.iter().all(|r| r.replacement.is_ascii_lowercase()));
    }

    #[test]
    fn genuine_foreign_text_not_flagged() {
        // Russian text, Greek symbols in English text, accented Latin.
        assert!(normalize("Москва — столица России, это очень большой город.", &[]).replaced.is_empty());
        assert!(normalize("The α-helix has a β value of 0.5 in the café.", &[]).replaced.is_empty());
    }

    #[test]
    fn invisible_chars_are_dropped_without_splitting() {
        let n = normalize("pla\u{200B}gia\u{200D}rism is\u{00AD}bad", &[]);
        assert_eq!(n.token_hashes(), hashes("plagiarism isbad"));
        assert_eq!(n.invisible.len(), 3);
        assert_eq!(n.invisible[0].pos, 3);
        assert_eq!(n.invisible[2].kind, InvisibleKind::SoftHyphen);
        // Original offsets preserved: token spans include the invisible char.
        assert_eq!(n.tokens[0].span(), Span::new(0, 12));
    }

    #[test]
    fn leading_bom_is_not_flagged() {
        let n = normalize("\u{FEFF}hello", &[]);
        assert!(n.invisible.is_empty());
        assert_eq!(n.tokens.len(), 1);
    }

    #[test]
    fn hidden_spans_separate_tokens() {
        // "theXquickXfox" where the X's are hidden (white text replacing spaces).
        let n = normalize("theXquickXfox", &[Span::new(3, 4), Span::new(9, 10)]);
        assert_eq!(n.token_hashes(), hashes("the quick fox"));
        assert_eq!(n.tokens[1].span(), Span::new(4, 9));
    }

    #[test]
    fn cjk_tokens_are_single_chars() {
        let n = normalize("抄袭检测 abc", &[]);
        assert_eq!(n.tokens.len(), 5);
    }

    #[test]
    fn countable_prefix_excludes_space_hidden_invisible() {
        let n = normalize("ab c\u{200B}d XY", &[Span::new(7, 9)]);
        let p = n.countable_prefix();
        assert_eq!(*p.last().unwrap(), 4); // a b c d
        assert_eq!(n.tokens_in(Span::new(0, 6)), 2);
    }

    #[test]
    fn combining_mark_insertion_is_ignored() {
        assert_eq!(hashes("pla\u{0301}giarism"), hashes("plagiarism"));
    }
}
