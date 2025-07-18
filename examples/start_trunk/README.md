<picture>
    <source srcset="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_Solid_White.svg" media="(prefers-color-scheme: dark)">
    <img src="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_RGB.svg" alt="Leptos Logo">
</picture>

# Leptos Client-Side Rendered (CSR) App

## Developing your Leptos CSR project

To develop your Leptos CSR project, running

```sh
thaw serve csr
```

will open your app in your default browser at `http://localhost:6321`.

## Deploying your Leptos CSR project

To build a Leptos CSR app for release, use the command

```sh
thaw build csr --release
```

This will output the files necessary to run your app into the `dist` folder; you can then use any static site host to serve these files.

For further information about hosting Leptos CSR apps, please refer to [the Leptos Book chapter on deployment available here][deploy-csr].

[deploy-csr]: https://book.leptos.dev/deployment/csr.html
