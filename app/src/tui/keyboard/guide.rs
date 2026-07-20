pub(in crate::tui) use crate::sound_check_guide::SoundCheckGuidePresentation as KeyboardNoteGuidePresentation;

pub(crate) type KeyboardNoteGuide = crate::sound_check_guide::SoundCheckGuide;

pub(in crate::tui) const KEYBOARD_NOTE_GUIDE_MESSAGE: &str =
    "c,d,e,f,g,a,bキーを押して音が鳴ることを確認してください";

pub(super) use crate::sound_check_guide::local_date_string;
