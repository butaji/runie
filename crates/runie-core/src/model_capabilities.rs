use super::{InputKind, Model};

impl Model {
    pub fn supports_input(&self, kind: InputKind) -> bool {
        self.input.contains(&kind)
    }

    pub fn supports_images(&self) -> bool {
        self.supports_input(InputKind::Image)
    }
}
