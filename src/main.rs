mod leaflet;
mod point_display;

use yew::prelude::*;

use crate::leaflet::MapComponent;

const SERVER_ADDR: &'static str = "http://localhost:8080";

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <main style="height: 100%" class="">
            <nav class="navbar bg-body-tertiary">
                <div class="container-fluid">
                    <span class="navbar-brand mb-0 h1">{"Navbar"}</span>
                </div>
            </nav>

            <MapComponent style="height: 100%"/>
        </main>
    }
}
