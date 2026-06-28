use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, PartialEq)]
pub enum EmojiSeg {
    Text(String),
    Emoji { url: String },
}

const TWEMOJI_BASE: &str = "https://cdn.jsdelivr.net/gh/jdecked/twemoji@15.1.0/assets/72x72";

pub fn custom_emoji_url(id: &str) -> String {
    format!("https://cdn.discordapp.com/emojis/{id}.png?size=44")
}

/// Code twemoji : codepoints hex minuscule joints par '-', FE0F retiré sauf si ZWJ présent.
pub fn twemoji_code(grapheme: &str) -> String {
    let has_zwj = grapheme.contains('\u{200D}');
    grapheme
        .chars()
        .filter(|&c| has_zwj || c != '\u{FE0F}')
        .map(|c| format!("{:x}", c as u32))
        .collect::<Vec<_>>()
        .join("-")
}

pub fn twemoji_url(grapheme: &str) -> String {
    format!("{TWEMOJI_BASE}/{}.png", twemoji_code(grapheme))
}

/// Un graphème est un emoji Unicode s'il contient VS16 (U+FE0F) ou si son 1er
/// caractère a la propriété Emoji_Presentation.
fn is_emoji_grapheme(g: &str) -> bool {
    g.contains('\u{FE0F}')
        || g.chars()
            .next()
            .map(unic_emoji_char::is_emoji_presentation)
            .unwrap_or(false)
}

/// Essaie de parser un emoji custom `<:nom:id>` / `<a:nom:id>` au début de `s`.
/// Renvoie (octets consommés, id) si match.
fn parse_custom(s: &str) -> Option<(usize, String)> {
    let b = s.as_bytes();
    if b.first() != Some(&b'<') {
        return None;
    }
    let mut i = 1;
    if b.get(i) == Some(&b'a') {
        i += 1; // animé
    }
    if b.get(i) != Some(&b':') {
        return None;
    }
    i += 1;
    let name_start = i;
    while i < b.len() && b[i] != b':' && b[i] != b'>' {
        i += 1;
    }
    if i == name_start || b.get(i) != Some(&b':') {
        return None;
    }
    i += 1;
    let id_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == id_start || b.get(i) != Some(&b'>') {
        return None;
    }
    Some((i + 1, s[id_start..i].to_string()))
}

/// Découpe une chaîne en segments texte/emoji (custom + Unicode).
pub fn split_emojis(input: &str) -> Vec<EmojiSeg> {
    let mut segs = Vec::new();
    let mut buf = String::new();
    let mut rest = input;
    while !rest.is_empty() {
        if let Some((consumed, id)) = parse_custom(rest) {
            if !buf.is_empty() {
                segs.push(EmojiSeg::Text(std::mem::take(&mut buf)));
            }
            segs.push(EmojiSeg::Emoji {
                url: custom_emoji_url(&id),
            });
            rest = &rest[consumed..];
            continue;
        }
        let g = rest.graphemes(true).next().unwrap();
        if is_emoji_grapheme(g) {
            if !buf.is_empty() {
                segs.push(EmojiSeg::Text(std::mem::take(&mut buf)));
            }
            segs.push(EmojiSeg::Emoji {
                url: twemoji_url(g),
            });
        } else {
            buf.push_str(g);
        }
        rest = &rest[g.len()..];
    }
    if !buf.is_empty() {
        segs.push(EmojiSeg::Text(buf));
    }
    segs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_url() {
        assert_eq!(
            custom_emoji_url("123"),
            "https://cdn.discordapp.com/emojis/123.png?size=44"
        );
    }

    #[test]
    fn twemoji_code_simple_et_fe0f_et_zwj() {
        assert_eq!(twemoji_code("\u{1F600}"), "1f600"); // 😀
        assert_eq!(twemoji_code("\u{2764}\u{FE0F}"), "2764"); // ❤️ : FE0F retiré (pas de ZWJ)
                                                              // 👨‍👩‍👧 : ZWJ présent → tous les codepoints gardés
        assert_eq!(
            twemoji_code("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"),
            "1f468-200d-1f469-200d-1f467"
        );
        // keycap 5️⃣ : 0035 FE0F 20E3, pas de ZWJ → FE0F retiré
        assert_eq!(twemoji_code("5\u{FE0F}\u{20E3}"), "35-20e3");
    }

    #[test]
    fn split_custom_simple_et_anime() {
        assert_eq!(
            split_emojis("<:smile:123>"),
            vec![EmojiSeg::Emoji {
                url: custom_emoji_url("123")
            }]
        );
        assert_eq!(
            split_emojis("<a:wave:456>"),
            vec![EmojiSeg::Emoji {
                url: custom_emoji_url("456")
            }]
        );
    }

    #[test]
    fn split_unicode_mixte() {
        let segs = split_emojis("a\u{1F600}b");
        assert_eq!(
            segs,
            vec![
                EmojiSeg::Text("a".into()),
                EmojiSeg::Emoji {
                    url: twemoji_url("\u{1F600}")
                },
                EmojiSeg::Text("b".into()),
            ]
        );
    }

    #[test]
    fn chiffre_nu_n_est_pas_emoji() {
        assert_eq!(split_emojis("5"), vec![EmojiSeg::Text("5".into())]);
    }

    #[test]
    fn keycap_est_emoji() {
        let segs = split_emojis("5\u{FE0F}\u{20E3}");
        assert_eq!(
            segs,
            vec![EmojiSeg::Emoji {
                url: twemoji_url("5\u{FE0F}\u{20E3}")
            }]
        );
    }

    #[test]
    fn texte_consecutif_fusionne() {
        assert_eq!(split_emojis("hello"), vec![EmojiSeg::Text("hello".into())]);
    }
}
