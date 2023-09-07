mod leaflet;

use yew::prelude::*;

use crate::leaflet::MapComponent;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <main>
            <h1>{ "Hello World!" }</h1>
            <MapComponent style="height: 500px"/>
        </main>
    }
}
