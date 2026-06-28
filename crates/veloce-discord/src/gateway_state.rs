#[derive(Debug, Clone, PartialEq)]
pub enum GatewayAction {
    StartHeartbeat {
        interval_ms: u64,
    },
    SendHeartbeat,
    Identify,
    Resume {
        session_id: String,
        seq: Option<u64>,
    },
    ReconnectResumable,
    ReconnectFull,
    Dispatch(String),
    Ignore,
}

#[derive(Debug, Clone, Default)]
pub struct GatewayState {
    pub seq: Option<u64>,
    pub session_id: Option<String>,
    pub heartbeat_interval_ms: Option<u64>,
    pub last_ack: bool,
}

impl GatewayState {
    pub fn on_hello(&mut self, interval_ms: u64) -> GatewayAction {
        self.heartbeat_interval_ms = Some(interval_ms);
        self.last_ack = true;
        GatewayAction::StartHeartbeat { interval_ms }
    }

    pub fn handshake_action(&self) -> GatewayAction {
        match &self.session_id {
            Some(id) => GatewayAction::Resume {
                session_id: id.clone(),
                seq: self.seq,
            },
            None => GatewayAction::Identify,
        }
    }

    pub fn set_session(&mut self, id: String) {
        self.session_id = Some(id);
    }

    pub fn on_payload(
        &mut self,
        op: u8,
        t: Option<&str>,
        s: Option<u64>,
        invalid_session_resumable: Option<bool>,
    ) -> GatewayAction {
        match op {
            0 => {
                if let Some(seq) = s {
                    self.seq = Some(seq);
                }
                GatewayAction::Dispatch(t.unwrap_or_default().to_string())
            }
            1 => GatewayAction::SendHeartbeat,
            7 => GatewayAction::ReconnectResumable,
            9 => {
                if invalid_session_resumable == Some(true) {
                    GatewayAction::ReconnectResumable
                } else {
                    self.session_id = None;
                    self.seq = None;
                    GatewayAction::ReconnectFull
                }
            }
            11 => {
                self.last_ack = true;
                GatewayAction::Ignore
            }
            _ => GatewayAction::Ignore,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_demarre_heartbeat() {
        let mut s = GatewayState::default();
        assert_eq!(
            s.on_hello(41250),
            GatewayAction::StartHeartbeat { interval_ms: 41250 }
        );
        assert_eq!(s.heartbeat_interval_ms, Some(41250));
    }

    #[test]
    fn handshake_identify_sans_session_puis_resume_avec() {
        let mut s = GatewayState::default();
        assert_eq!(s.handshake_action(), GatewayAction::Identify);
        s.set_session("sess-1".into());
        s.on_payload(0, Some("MESSAGE_CREATE"), Some(7), None);
        assert_eq!(
            s.handshake_action(),
            GatewayAction::Resume {
                session_id: "sess-1".into(),
                seq: Some(7)
            }
        );
    }

    #[test]
    fn dispatch_enregistre_la_sequence() {
        let mut s = GatewayState::default();
        let a = s.on_payload(0, Some("READY"), Some(1), None);
        assert_eq!(a, GatewayAction::Dispatch("READY".into()));
        assert_eq!(s.seq, Some(1));
    }

    #[test]
    fn op1_demande_heartbeat() {
        let mut s = GatewayState::default();
        assert_eq!(
            s.on_payload(1, None, None, None),
            GatewayAction::SendHeartbeat
        );
    }

    #[test]
    fn invalid_session_non_resumable_vide_la_session() {
        let mut s = GatewayState::default();
        s.set_session("x".into());
        let a = s.on_payload(9, None, None, Some(false));
        assert_eq!(a, GatewayAction::ReconnectFull);
        assert!(s.session_id.is_none());
    }

    #[test]
    fn invalid_session_resumable_garde_la_session() {
        let mut s = GatewayState::default();
        s.set_session("x".into());
        let a = s.on_payload(9, None, None, Some(true));
        assert_eq!(a, GatewayAction::ReconnectResumable);
        assert_eq!(s.session_id.as_deref(), Some("x"));
    }

    #[test]
    fn ack_met_a_jour_last_ack() {
        let mut s = GatewayState {
            last_ack: false,
            ..Default::default()
        };
        assert_eq!(s.on_payload(11, None, None, None), GatewayAction::Ignore);
        assert!(s.last_ack);
    }
}
