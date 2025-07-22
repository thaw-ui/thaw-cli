<picture>
    <source srcset="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_Solid_White.svg" media="(prefers-color-scheme: dark)">
    <img src="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_RGB.svg" alt="Leptos Logo">
</picture>

# Leptos Server-Side Rendering (CSR) App (Axum)

## Developing your Leptos SSR project

To develop your Leptos CSR project, running

```sh
thaw serve ssr
```

will open your app in your default browser at `http://localhost:6321`.

## Deploying your Leptos SSR project

To build a Leptos SSR app for release, use the command

```sh
thaw build ssr --release
```

This will output the files necessary to run your app into the `dist` folder.

