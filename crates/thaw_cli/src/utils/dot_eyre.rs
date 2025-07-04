pub trait DotEyre {
    type Output;
    fn dot_eyre(self) -> color_eyre::Result<Self::Output>;
}

impl<T> DotEyre for anyhow::Result<T> {
    type Output = T;

    fn dot_eyre(self) -> color_eyre::Result<T> {
        match self {
            Ok(value) => color_eyre::Result::Ok(value),
            Err(err) => {
                // println!("err {:#?}", err.backtrace());
                // https://github.com/eyre-rs/eyre/issues/31#issuecomment-2558379257
                let boxed_error = Box::<dyn std::error::Error + Send + Sync + 'static>::from(err);
                let report = color_eyre::eyre::eyre!(boxed_error);
                color_eyre::Result::Err(report)?
            }
        }
    }
}
