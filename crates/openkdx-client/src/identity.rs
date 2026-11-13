use openkdx_proto::msg::common::{Color, UserRecord};

/// What the client says about itself in 100 Set Identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub nick: Vec<u8>,
    pub info: Vec<u8>,
    pub text_color: Color,
    pub background_color: Color,
    /// 2 bits per pixel user icon, or empty.
    pub icon: Vec<u8>,
    /// Present as a registered Client 1.600: send 118 after 100.
    pub registered: bool,
}

impl Default for Identity {
    fn default() -> Self {
        Identity {
            nick: b"openkdx".to_vec(),
            info: Vec::new(),
            text_color: Color { r: 0, g: 0, b: 0 },
            background_color: Color {
                r: 0xff,
                g: 0xcc,
                b: 0x00,
            },
            icon: Vec::new(),
            registered: true,
        }
    }
}

impl Identity {
    pub fn user_record(&self) -> UserRecord {
        UserRecord {
            user_id: 0,
            status: 0,
            ignored_05: [0; 5],
            client_port: 10600,
            text_color: self.text_color,
            background_color: self.background_color,
            account_color: Color { r: 0, g: 0, b: 0 },
            flags: 0,
            ignored_1b: 0,
            icon: self.icon.clone(),
            extra: Vec::new(),
            nick: self.nick.clone(),
            info: self.info.clone(),
            padding: Vec::new(),
        }
    }
}
