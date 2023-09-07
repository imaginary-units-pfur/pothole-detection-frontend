use gloo_utils::document;
use leaflet::{LatLng, Map, TileLayer};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{HtmlElement, Node};
use yew::prelude::*;
use yew_hooks::{use_effect_once, use_is_first_mount};

#[derive(PartialEq, Properties, Clone)]
pub struct Props {
    #[prop_or_default]
    pub class: Classes,
    #[prop_or_default]
    pub style: AttrValue,
}

#[function_component]
pub fn MapComponent(props: &Props) -> Html {
    let leaflet_box = use_state(|| None);
    let container_box = use_state(|| None);

    let (leaflet, container) = if use_is_first_mount() {
        // Initialize the target HTML element
        let container = document().create_element("div").unwrap();
        let container: HtmlElement = container.dyn_into().unwrap();
        container.set_class_name("map");
        let leaflet_map = Map::new_with_element(&container, &JsValue::NULL);

        leaflet_map.setView(&LatLng::new(12.34, 56.78), 11.0);
        add_tile_layer(&leaflet_map);

        container_box.set(Some(container.clone()));
        leaflet_box.set(Some(leaflet_map));

        // let map = leaflet_box.as_ref().unwrap();
        // (map, container)

        // Turns out that setting a value into a UseStateHandle doesn't happen immediately.
        // So, for the first render, return nothing
        // Because we set the UseStateHandle, a rerender will happen immediately.
        return html!();
    } else {
        let map = leaflet_box.as_ref().unwrap();
        let container = (*container_box.as_ref().unwrap()).clone();
        (map, container)
    };

    // To render the map, need to create VRef to the map's element
    let node: &Node = &container.clone().into();
    html! {
        <div class={classes!("map-container", props.class.clone())} style={&props.style}>
            {Html::VRef(node.clone())}
        </div>
    }
}

fn add_tile_layer(map: &Map) {
    TileLayer::new("http://localhost:3000/_/{z}/{x}/{y}.png", &JsValue::NULL).addTo(map);
}
