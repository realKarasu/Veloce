/// Convertit une date civile (UTC) en jours depuis 1970-01-01 (algorithme de
/// Howard Hinnant, pur entier).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Parse les composants `YYYY-MM-DDTHH:MM:SS` au début de `iso` (la fraction et
/// le fuseau sont ignorés ; Discord renvoie de l'UTC). Renvoie (y,mo,d,h,mi,s).
fn parts(iso: &str) -> Option<(i64, i64, i64, i64, i64, i64)> {
    let b = iso.as_bytes();
    if b.len() < 19
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let num = |a: usize, z: usize| iso.get(a..z)?.parse::<i64>().ok();
    Some((
        num(0, 4)?,
        num(5, 7)?,
        num(8, 10)?,
        num(11, 13)?,
        num(14, 16)?,
        num(17, 19)?,
    ))
}

pub fn parse_epoch(iso: &str) -> Option<i64> {
    let (y, mo, d, h, mi, s) = parts(iso)?;
    Some(days_from_civil(y, mo, d) * 86400 + h * 3600 + mi * 60 + s)
}

use chrono::{DateTime, Local};

/// Heure locale (DST correct) pour l'affichage.
pub fn format_timestamp(iso: &str) -> String {
    match DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => dt
            .with_timezone(&Local)
            .format("%d/%m/%Y à %H:%M")
            .to_string(),
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    /// Pur et testable : formate `iso` (RFC3339) dans le décalage `offset`.
    fn format_with_offset(iso: &str, offset: FixedOffset) -> String {
        match DateTime::parse_from_rfc3339(iso) {
            Ok(dt) => dt
                .with_timezone(&offset)
                .format("%d/%m/%Y à %H:%M")
                .to_string(),
            Err(_) => String::new(),
        }
    }

    #[test]
    fn epoch_connu() {
        // 1970-01-01T00:00:00 = 0
        assert_eq!(parse_epoch("1970-01-01T00:00:00.000000+00:00"), Some(0));
        // 2021-01-01T00:00:00Z = 1609459200
        assert_eq!(
            parse_epoch("2021-01-01T00:00:00+00:00"),
            Some(1_609_459_200)
        );
    }

    #[test]
    fn epoch_invalide_donne_none() {
        assert_eq!(parse_epoch(""), None);
        assert_eq!(parse_epoch("pas-une-date"), None);
    }

    #[test]
    fn format_avec_offset_utc() {
        assert_eq!(
            format_with_offset(
                "2026-06-29T14:23:45.000000+00:00",
                FixedOffset::east_opt(0).unwrap(),
            ),
            "29/06/2026 à 14:23"
        );
    }

    #[test]
    fn format_avec_offset_utc_plus_2() {
        assert_eq!(
            format_with_offset(
                "2026-06-29T14:23:45.000000+00:00",
                FixedOffset::east_opt(2 * 3600).unwrap(),
            ),
            "29/06/2026 à 16:23"
        );
    }

    #[test]
    fn format_invalide_donne_vide() {
        assert_eq!(format_timestamp("xxx"), "");
        assert_eq!(
            format_with_offset("pas-une-date", FixedOffset::east_opt(0).unwrap()),
            ""
        );
    }
}
