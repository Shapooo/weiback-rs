#[derive(Default, Debug, Clone)]
pub struct SearchArgs {
    pub hasori: bool,
    pub hasret: bool,
    pub hastext: bool,
    pub haspic: bool,
    pub hasvideo: bool,
    pub hasmusic: bool,
}

impl SearchArgs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ori(mut self) -> Self {
        self.hasori = true;
        self
    }

    pub fn with_ret(mut self) -> Self {
        self.hasret = true;
        self
    }

    pub fn with_text(mut self) -> Self {
        self.hastext = true;
        self
    }

    pub fn with_pic(mut self) -> Self {
        self.haspic = true;
        self
    }

    pub fn with_video(mut self) -> Self {
        self.hasvideo = true;
        self
    }

    pub fn with_music(mut self) -> Self {
        self.hasmusic = true;
        self
    }

    pub fn attach_args(&self, mut base: String) -> String {
        if self.hasori {
            base.push_str("&hasori=1");
        }
        if self.hasret {
            base.push_str("&hasret=1");
        }
        if self.hastext {
            base.push_str("&hastext=1");
        }
        if self.haspic {
            base.push_str("&haspic=1");
        }
        if self.hasvideo {
            base.push_str("&hasvideo=1");
        }
        if self.hasmusic {
            base.push_str("&hasmusic=1");
        }
        base
    }
}
