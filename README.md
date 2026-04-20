# Vegan Recipe Search Engine

A WebAssembly-powered search engine for vegan recipes, built with Rust. The project parses recipe data from an XML file and renders it in the browser.

## Prerequisites

- **Rust** - Install via [rustup](https://rustup.rs/)
- **wasm-pack** - Install with `cargo install wasm-pack`
- **Python 3** - For running a local HTTP server

## Build

Compile the Rust code to WebAssembly:

```bash
wasm-pack build --target web
```

This generates the `pkg/` directory containing the WASM module and JavaScript bindings.

## Run

Start a local HTTP server:

```bash
python -m http.server
```

Then open http://localhost:8000 in your browser.

## Architecture

- **`search.xml`** - Recipe data source (title, author, image, link, tags, ingredients, seasons, tools)
- **`src/lib.rs`** - Rust logic: fetches XML, parses with serde-xml-rs, renders HTML
- **`index.html`** - Entry point that loads and runs the WASM module
- **`pkg/`** - Generated WebAssembly package (output of `wasm-pack build`)
