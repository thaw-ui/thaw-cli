pub fn cargo_build_args() -> Vec<&'static str> {
    vec![
        "--target=wasm32-unknown-unknown",
        "--lib",
        "--features=hydrate",
    ]
}
