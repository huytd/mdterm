mod app;
mod components;
pub mod html;
mod markdown;
mod state;
mod tauri_bridge;

use app::App;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(|| leptos::prelude::view! { <App /> });
}
