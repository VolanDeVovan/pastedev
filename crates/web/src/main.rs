// The Dioxus WASM SPA entry-point. `dx build --platform web` compiles this to
// `wasm32-unknown-unknown` and emits the bootstrap glue.

#![allow(non_snake_case)]

mod api;
mod app;
mod config;
mod components;
mod editor;
mod js;
mod lib_util;
mod pages;
mod route;
mod state;
mod views;

use dioxus::prelude::*;

fn main() {
    // Pretty panics in dev. Cheap (~1 kB compressed).
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
    dioxus::launch(app::App);
}
