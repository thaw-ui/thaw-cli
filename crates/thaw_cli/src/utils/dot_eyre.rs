use std::fmt::Debug;

pub trait DotEyre {
    type Output;
    fn dot_eyre(self) -> color_eyre::Result<Self::Output>;
}

impl<T, E: Debug> DotEyre for anyhow::Result<T, E> {
    type Output = T;

    fn dot_eyre(self) -> color_eyre::Result<T> {
        match self {
            Ok(value) => color_eyre::Result::Ok(value),
            Err(err) => {
                let report = color_eyre::eyre::eyre!("{:#?}", err);
                color_eyre::Result::Err(report)?
            }
        }
    }
}
