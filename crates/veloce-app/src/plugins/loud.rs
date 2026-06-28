use crate::plugins::Plugin;

#[derive(Default)]
pub struct Loud;

impl Plugin for Loud {
    fn name(&self) -> &str {
        "Loud"
    }
    fn description(&self) -> &str {
        "MET LES MESSAGES AFFICHÉS EN MAJUSCULES."
    }
    fn on_render_content(&self, content: &mut String) {
        *content = content.to_uppercase();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn met_en_majuscules() {
        let p = Loud;
        let mut s = "hello".to_string();
        p.on_render_content(&mut s);
        assert_eq!(s, "HELLO");
    }
}
