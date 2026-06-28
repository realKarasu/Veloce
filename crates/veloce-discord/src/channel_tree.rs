use crate::models::{Channel, Snowflake};
use std::collections::HashSet;

const CATEGORY: u8 = 4;

#[derive(Debug, Clone)]
pub enum TreeRow {
    Category { id: Snowflake, name: String },
    Channel(Channel),
}

/// 1 pour vocal/stage (affichés après), 0 sinon.
fn type_group(kind: u8) -> u8 {
    match kind {
        2 | 13 => 1,
        _ => 0,
    }
}

fn sort_key(c: &Channel) -> (u8, i32, String) {
    (type_group(c.kind), c.position.unwrap_or(0), c.id.clone())
}

/// Construit l'arbre fidèle Discord : salons racine, puis catégories + enfants.
pub fn build_channel_tree(channels: &[Channel], visible: &HashSet<Snowflake>) -> Vec<TreeRow> {
    let is_visible = |c: &Channel| visible.contains(&c.id);
    let mut rows = Vec::new();

    // Salons racine (pas de parent, pas une catégorie), visibles.
    let mut roots: Vec<&Channel> = channels
        .iter()
        .filter(|c| c.kind != CATEGORY && c.parent_id.is_none() && is_visible(c))
        .collect();
    roots.sort_by_key(|c| sort_key(c));
    for c in roots {
        rows.push(TreeRow::Channel(c.clone()));
    }

    // Catégories triées par position ; incluses seulement si ≥1 enfant visible.
    let mut cats: Vec<&Channel> = channels.iter().filter(|c| c.kind == CATEGORY).collect();
    cats.sort_by_key(|c| (c.position.unwrap_or(0), c.id.clone()));
    for cat in cats {
        let mut children: Vec<&Channel> = channels
            .iter()
            .filter(|c| {
                c.kind != CATEGORY
                    && c.parent_id.as_deref() == Some(cat.id.as_str())
                    && is_visible(c)
            })
            .collect();
        if children.is_empty() {
            continue;
        }
        children.sort_by_key(|c| sort_key(c));
        rows.push(TreeRow::Category {
            id: cat.id.clone(),
            name: cat.name.clone().unwrap_or_default(),
        });
        for c in children {
            rows.push(TreeRow::Channel(c.clone()));
        }
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Channel;
    use std::collections::HashSet;

    fn c(id: &str, kind: u8, parent: Option<&str>, pos: i32) -> Channel {
        Channel {
            id: id.into(),
            name: Some(id.into()),
            kind,
            guild_id: Some("10".into()),
            position: Some(pos),
            parent_id: parent.map(|s| s.into()),
            permission_overwrites: vec![],
        }
    }

    #[test]
    fn groupe_par_categorie_et_ordonne() {
        let chans = vec![
            c("cat1", 4, None, 1),
            c("txt", 0, Some("cat1"), 1),
            c("voc", 2, Some("cat1"), 0), // vocal -> après le texte malgré position plus basse
            c("root", 0, None, 0),        // salon racine sans catégorie
        ];
        let visible: HashSet<_> = ["cat1", "txt", "voc", "root"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let rows = build_channel_tree(&chans, &visible);
        // racine d'abord, puis catégorie, puis ses enfants (texte avant vocal)
        match &rows[0] {
            TreeRow::Channel(ch) => assert_eq!(ch.id, "root"),
            _ => panic!(),
        }
        match &rows[1] {
            TreeRow::Category { id, .. } => assert_eq!(id, "cat1"),
            _ => panic!(),
        }
        match &rows[2] {
            TreeRow::Channel(ch) => assert_eq!(ch.id, "txt"),
            _ => panic!(),
        }
        match &rows[3] {
            TreeRow::Channel(ch) => assert_eq!(ch.id, "voc"),
            _ => panic!(),
        }
    }

    #[test]
    fn filtre_invisibles_et_categorie_vide_masquee() {
        let chans = vec![
            c("cat1", 4, None, 0),
            c("secret", 0, Some("cat1"), 0), // pas dans visible
        ];
        let visible: HashSet<_> = HashSet::new(); // rien de visible
        let rows = build_channel_tree(&chans, &visible);
        assert!(rows.is_empty()); // catégorie sans enfant visible -> masquée
    }
}
