#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreAction {
    MenuUp,
    MenuDown,
    StartFromMenu,
    OpenExitPrompt,
    OpenAbandonPrompt,
    ConfirmPrompt,
    CancelPrompt,
    OptionsUp,
    OptionsDown,
    ToggleOptionOrStart,
    StartFromOptions,
    InputChar(char),
    Backspace,
    SubmitAnswer,
    ContinueAfterFeedback,
    ContinueAfterUnlock,
    FinishedToMenu,
    SetElapsedSeconds(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreEffect {
    PersistBestOfScore {
        correct: u32,
        incorrect: u32,
        elapsed_secs: u64,
        points: i64,
    },
}
