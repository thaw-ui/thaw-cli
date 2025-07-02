use cargo_metadata::diagnostic::Diagnostic;
use crossterm::{
    ExecutableCommand, QueueableCommand,
    style::{self, Stylize},
    terminal,
};
use std::{
    fmt,
    io::{self, Write},
};

#[derive(Debug)]
pub enum Message {
    CargoPackaging(CargoPackagingMessage),
    CargoBuildFinished,
}

impl Message {
    fn is_overlay_print(&self, last_message: &Option<Self>) -> bool {
        let Some(last_message) = last_message else {
            return false;
        };

        match (self, last_message) {
            (
                Self::CargoPackaging(CargoPackagingMessage::Compiling(_)),
                Self::CargoPackaging(CargoPackagingMessage::Compiling(_)),
            )
            | (
                Self::CargoPackaging(CargoPackagingMessage::CompilerMessage(_)),
                Self::CargoPackaging(CargoPackagingMessage::Compiling(_)),
            )
            | (
                Self::CargoBuildFinished,
                Self::CargoPackaging(CargoPackagingMessage::Compiling(_)),
            ) => true,
            (Self::CargoBuildFinished, Self::CargoBuildFinished) => unreachable!(),
            (_, _) => false,
        }
    }
}

#[derive(Debug)]
pub enum CargoPackagingMessage {
    Compiling(String),
    Warning(String),
    CompilerMessage(Diagnostic),
    Other(String),
}

impl fmt::Display for CargoPackagingMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::CompilerMessage(value) => write!(f, "{value}")?,
            Self::Compiling(value) | Self::Warning(value) | Self::Other(value) => {
                f.write_str(value)?
            }
        }
        Ok(())
    }
}

impl From<String> for CargoPackagingMessage {
    fn from(value: String) -> Self {
        if let Ok(diagnostic) = serde_json::from_str::<Diagnostic>(&value) {
            Self::CompilerMessage(diagnostic)
        } else if value.trim_start().starts_with("Compiling ") {
            let compiling = fmt::format(format_args!("{}", "Compiling".green()));
            Self::Compiling(value.replace("Compiling", &compiling))
        } else if value.trim_start().starts_with("warning:") {
            let warning = fmt::format(format_args!("{}", "warning".yellow()));
            Self::Warning(value.replace("warning", &warning))
        } else {
            Self::Other(value)
        }
    }
}

impl From<Diagnostic> for CargoPackagingMessage {
    fn from(value: Diagnostic) -> Self {
        Self::CompilerMessage(value)
    }
}

#[derive(Debug)]
pub struct PrintMessage {
    last_message: Option<Message>,
    stdout: io::Stdout,
}

impl PrintMessage {
    pub fn new() -> Self {
        Self {
            last_message: None,
            stdout: io::stdout(),
        }
    }

    pub fn print(&mut self, message: Message) -> color_eyre::Result<()> {
        if message.is_overlay_print(&self.last_message) {
            self.stdout
                .queue(terminal::Clear(terminal::ClearType::CurrentLine))?
                .queue(style::Print("\r"))?
                .flush()?;
        } else {
            self.stdout.execute(style::Print("\n"))?;
        }

        match &message {
            Message::CargoPackaging(message) => {
                self.stdout.execute(style::Print(message))?;
            }
            Message::CargoBuildFinished => {
                self.stdout
                    .execute(style::Print("Compilation completed."))?;
            }
        }

        self.last_message = Some(message);
        Ok(())
    }
}
