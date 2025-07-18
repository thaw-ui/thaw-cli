use cargo_metadata::diagnostic::Diagnostic;
use crossterm::{
    ExecutableCommand, QueueableCommand,
    style::{self, Stylize},
    terminal,
};
use std::{
    fmt,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Message {
    CargoPackaging(CargoPackagingMessage),
    CargoBuildFinished,
    Build(String),
    InitBuildFinished,
    PageReload(Vec<PathBuf>, color_eyre::Result<()>),
}

impl Message {
    fn is_overlay_print(&self, last_message: &Option<Self>) -> bool {
        let Some(last_message) = last_message else {
            return false;
        };

        match (last_message, self) {
            // Blocking
            // Blocking
            (
                Self::CargoPackaging(CargoPackagingMessage::Blocking(_)),
                Self::CargoPackaging(CargoPackagingMessage::Blocking(_)),
            )
            // Blocking
            // Compiling
            | (
                Self::CargoPackaging(CargoPackagingMessage::Blocking(_)),
                Self::CargoPackaging(CargoPackagingMessage::Compiling(_)),
            )
            // Compiling
            // Compiling
            | (
                Self::CargoPackaging(CargoPackagingMessage::Compiling(_)),
                Self::CargoPackaging(CargoPackagingMessage::Compiling(_)),
            )
            // Compiling
            // error: 
            | (
                Self::CargoPackaging(CargoPackagingMessage::Compiling(_)),
                Self::CargoPackaging(CargoPackagingMessage::CompilerMessage(_)),
            )
            // Compiling
            // Finished
            | (
                Self::CargoPackaging(CargoPackagingMessage::Compiling(_)),
                Self::CargoPackaging(CargoPackagingMessage::Finished(_)),
            )
            // CargoBuildFinished
            // Finished
            | (
                Self::CargoBuildFinished,
                Self::CargoPackaging(CargoPackagingMessage::Finished(_)),
            )
            | (
                Self::CargoPackaging(CargoPackagingMessage::Finished(_)),
                Self::Build(_),
            )
            | (
                Self::Build(_),
                Self::Build(_),
            )
            // Finished
            // PageReload
            | (
                Self::Build(_),
                Self::PageReload(_, _),
            )=> true,
            (Self::CargoBuildFinished, Self::CargoBuildFinished) => unreachable!(),
            (_, _) => false,
        }
    }

    fn is_wrap(&self, last_message: &Option<Self>) -> bool {
        let Some(last_message) = last_message else {
            return false;
        };

        match (last_message, self) {
            (_, Self::CargoBuildFinished)
            | (Self::CargoPackaging(CargoPackagingMessage::CompilerMessage(_)), _) => false,
            (_, _) => true,
        }
    }
}

#[derive(Debug)]
pub enum CargoPackagingMessage {
    Blocking(String),
    Compiling(String),
    Warning(String),
    Finished(String),
    CompilerMessage(Diagnostic),
    Other(String),
}

impl fmt::Display for CargoPackagingMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::CompilerMessage(value) => write!(f, "{value}")?,
            Self::Blocking(value)
            | Self::Compiling(value)
            | Self::Warning(value)
            | Self::Finished(value)
            | Self::Other(value) => f.write_str(value)?,
        }
        Ok(())
    }
}

impl From<String> for CargoPackagingMessage {
    fn from(value: String) -> Self {
        if let Ok(diagnostic) = serde_json::from_str::<Diagnostic>(&value) {
            return Self::CompilerMessage(diagnostic);
        }

        let trim_start = value.trim_start();
        if trim_start.starts_with("Compiling ") {
            let compiling = fmt::format(format_args!("{}", "Compiling".green()));
            Self::Compiling(trim_start.replace("Compiling", &compiling))
        } else if trim_start.starts_with("Blocking ") {
            let blocking = fmt::format(format_args!("{}", "Blocking".cyan()));
            Self::Blocking(trim_start.replace("Blocking", &blocking))
        } else if trim_start.starts_with("Finished ") {
            let finished = fmt::format(format_args!("{}", "Finished".green()));
            Self::Finished(trim_start.replace("Finished", &finished))
        } else if trim_start.starts_with("warning:") {
            let warning = fmt::format(format_args!("{}", "warning".yellow()));
            Self::Warning(trim_start.replace("warning", &warning))
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
    current_dir: PathBuf,
}

impl PrintMessage {
    pub fn new(current_dir: PathBuf) -> Self {
        Self {
            last_message: None,
            stdout: io::stdout(),
            current_dir,
        }
    }

    pub fn print(&mut self, message: Message) -> color_eyre::Result<()> {
        if let Message::InitBuildFinished = message {
            self.last_message = None;
            return Ok(());
        }
        if message.is_overlay_print(&self.last_message) {
            self.stdout
                .queue(terminal::Clear(terminal::ClearType::CurrentLine))?
                .queue(style::Print("\r"))?
                .flush()?;
        } else if message.is_wrap(&self.last_message) {
            self.stdout.execute(style::Print("\n"))?;
        }

        match &message {
            Message::CargoPackaging(message) => {
                self.stdout.execute(style::Print(message))?;
            }
            Message::Build(message) => {
                self.stdout.execute(style::Print(message))?;
            }
            Message::PageReload(paths, build_result) => {
                for path in paths {
                    let now = chrono::Local::now();
                    let message = match build_result {
                        Ok(_) => format!(
                            "{} {} {} {}",
                            now.format("%H:%M:%S"),
                            "[thaw-cli]".cyan(),
                            "page reload".green(),
                            normalize_path(path.strip_prefix(&self.current_dir).unwrap_or(path))
                        ),
                        Err(err) => format!(
                            "{} {} error: {err:?}",
                            now.format("%H:%M:%S"),
                            "[thaw-cli]".red(),
                        ),
                    };
                    self.stdout.execute(style::Print(message))?;
                }
            }
            Message::CargoBuildFinished => {}
            _ => unreachable!(),
        }

        // if matches!(
        //     message,
        //     Message::CargoPackaging(CargoPackagingMessage::Finished(_))
        // ) {
        //     self.stdout.execute(style::Print("\n"))?;
        // }

        self.last_message = Some(message);
        Ok(())
    }
}

fn normalize_path(path: &Path) -> String {
    let mut path = path.display().to_string();
    if cfg!(windows) {
        path = path.replacen("\\", "/", usize::MAX);
    }
    path
}
