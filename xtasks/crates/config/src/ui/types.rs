use crossterm::event::KeyEvent;
use enum_dispatch::enum_dispatch;
use ratatui::Frame;

use crate::{
    error::Result,
    state::ConfigState,
    ui::{
        confirm::{ExitConfirmationModal, SaveConfirmationModal},
        editor::EditorModal,
        help::HelpModal,
    },
};

#[enum_dispatch(Modallike)]
pub enum Modal {
    SaveConfirmation(SaveConfirmationModal),
    Editor(EditorModal),
    Help(HelpModal),
    ExitConfirmation(ExitConfirmationModal),
}

pub enum ModalCmd {
    ExitApp,
    Nothing,
    Close,
    Swap(Modal),
}

#[enum_dispatch]
pub trait Modallike {
    fn handle_key_event(&mut self, key: KeyEvent, state: &mut ConfigState) -> Result<ModalCmd>;
    fn draw(&mut self, f: &mut Frame);
    fn footer_text(&self) -> String;
}
