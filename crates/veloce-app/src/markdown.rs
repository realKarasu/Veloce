#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
}

impl Span {
    fn neutral(text: String) -> Self {
        Span {
            text,
            bold: false,
            italic: false,
            strike: false,
            code: false,
        }
    }
}

/// Marqueurs reconnus (ordre = priorité de détection).
#[allow(clippy::type_complexity)]
const MARKERS: &[(&str, fn(&mut Span))] = &[
    ("**", |s| s.bold = true),
    ("~~", |s| s.strike = true),
    ("`", |s| s.code = true),
    ("*", |s| s.italic = true),
];

pub fn parse_markdown(input: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut buf = String::new();
    let mut rest = input;

    'outer: while !rest.is_empty() {
        for (marker, apply) in MARKERS {
            if let Some(after_open) = rest.strip_prefix(marker) {
                if let Some(end) = after_open.find(marker) {
                    let content = &after_open[..end];
                    if !content.is_empty() {
                        if !buf.is_empty() {
                            spans.push(Span::neutral(std::mem::take(&mut buf)));
                        }
                        let mut span = Span::neutral(content.to_string());
                        apply(&mut span);
                        spans.push(span);
                        rest = &after_open[end + marker.len()..];
                        continue 'outer;
                    }
                }
            }
        }
        // aucun marqueur : consommer un caractère
        let ch = rest.chars().next().unwrap();
        buf.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    if !buf.is_empty() {
        spans.push(Span::neutral(buf));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn neutre(t: &str) -> Span {
        Span {
            text: t.into(),
            bold: false,
            italic: false,
            strike: false,
            code: false,
        }
    }

    #[test]
    fn texte_simple_un_span_neutre() {
        assert_eq!(parse_markdown("bonjour"), vec![neutre("bonjour")]);
    }

    #[test]
    fn gras_detecte() {
        let r = parse_markdown("a **b** c");
        assert_eq!(r[0], neutre("a "));
        assert_eq!(
            r[1],
            Span {
                text: "b".into(),
                bold: true,
                italic: false,
                strike: false,
                code: false
            }
        );
        assert_eq!(r[2], neutre(" c"));
    }

    #[test]
    fn italique_et_code() {
        let r = parse_markdown("*i* `c`");
        assert!(r.iter().any(|s| s.text == "i" && s.italic));
        assert!(r.iter().any(|s| s.text == "c" && s.code));
    }

    #[test]
    fn barre_detecte() {
        let r = parse_markdown("~~x~~");
        assert_eq!(
            r,
            vec![Span {
                text: "x".into(),
                bold: false,
                italic: false,
                strike: true,
                code: false
            }]
        );
    }

    #[test]
    fn marqueur_non_ferme_reste_litteral() {
        assert_eq!(parse_markdown("**oops"), vec![neutre("**oops")]);
    }
}
