use crate::context::Context;

/// Reads the BROWSER environment variable and decides what to do with it.
pub fn open_browser(context: &Context, url: String) -> color_eyre::Result<()> {
    let browser = std::env::var("BROWSER")
        .ok()
        .or(context.env.get("BROWSER").cloned())
        .unwrap_or_default();
    if browser.to_lowercase() != "none" {
        if browser.is_empty() {
            open::that(url)?;
        } else {
            open::with(url, browser)?;
        }
    }
    Ok(())
}

// const SUPPORTED_CHROMIUM_BROWSERS: [&str; 8] = [
//     "Google Chrome Canary",
//     "Google Chrome Dev",
//     "Google Chrome Beta",
//     "Google Chrome",
//     "Microsoft Edge",
//     "Brave Browser",
//     "Vivaldi",
//     "Chromium",
// ];
