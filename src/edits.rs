use crate::api::{Render, Sequence};

#[derive(Clone)]
pub enum Edit {
    Render(Render),
    Sequence(Sequence),
}

#[derive(Default)]
pub struct EditStack {
    undo: Vec<Edit>,
    redo: Vec<Edit>,
}

impl EditStack {
    pub fn push(&mut self, edit: Edit) {
        self.undo.push(edit);
        if self.undo.len() > 80 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn pop_undo(&mut self) -> Option<Edit> {
        self.undo.pop()
    }

    pub fn pop_redo(&mut self) -> Option<Edit> {
        self.redo.pop()
    }

    pub fn push_redo(&mut self, edit: Edit) {
        self.redo.push(edit);
    }

    #[allow(dead_code)]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
}
