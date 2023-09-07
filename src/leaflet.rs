mod icons;

use std::rc::Rc;

use gloo_utils::document;
use leaflet::{LatLng, Map, TileLayer};
use rand::{Rng, SeedableRng};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{HtmlElement, Node};
use yew::prelude::*;
use yew_hooks::use_is_first_mount;

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
    let markers = yew_hooks::use_list(vec![]);

    let (leaflet, container) = if use_is_first_mount() {
        // Initialize the target HTML element
        let container = document().create_element("div").unwrap();
        let container: HtmlElement = container.dyn_into().unwrap();
        container.set_class_name("map");
        let leaflet_map = Map::new_with_element(&container, &JsValue::NULL);

        leaflet_map.setView(&LatLng::new(55.34, 37.78), 11.0);
        add_tile_layer(&leaflet_map);

        container_box.set(Some(container.clone()));
        let leaflet_map = Rc::new(leaflet_map);

        // Populate the map
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        for idx in 0..10000 {
            let pos = LatLng::new(55.0 + rng.gen::<f64>() * 2.0, 37.0 + rng.gen::<f64>() * 2.0);
            let marker = leaflet::Marker::new(&pos);

            let handler = {
                let leaflet = leaflet_map.clone();
                move |e: leaflet::MouseEvent| {
                    e.originalEvent().prevent_default();
                    log::info!("Clicked on marker {idx}");
                    leaflet.zoomIn(2.0);
                }
            };
            let handler: Closure<dyn FnMut(leaflet::MouseEvent) -> ()> = Closure::new(handler);
            marker.on("click", &handler.into_js_value());
            markers.push((marker, pos, false));
        }

        let handler: Closure<dyn FnMut(leaflet::MouseEvent) -> ()> = Closure::new({
            let leaflet = leaflet_map.clone();
            let markers = markers.clone();

            move |e: leaflet::MouseEvent| {
                //e.originalEvent().prevent_default();
                let bounds = leaflet.getBounds();
                log::info!("Moving map: current bounds are {:?}", bounds);

                // Compute the markers that are to be updated
                fn clone_pos(l: &LatLng) -> LatLng {
                    LatLng::new(l.lat(), l.lng())
                }

                let mut markers_value = markers
                    .current()
                    .iter()
                    .map(|(m, l, b)| (m.clone(), clone_pos(l), *b))
                    .collect::<Vec<_>>();
                let mut to_add = vec![];
                let mut to_remove = vec![];
                for (idx, (_marker, pos, is_present)) in markers_value.iter().enumerate() {
                    let is_visible = bounds.contains(&pos);
                    match (is_visible, is_present) {
                        (true, false) => {
                            // Is not on map, but should be
                            to_add.push(idx);
                        }
                        (false, true) => {
                            // Is on map, but shouldn't be
                            to_remove.push(idx);
                        }
                        _ => {}
                    }
                }

                // Apply changes
                for idx in to_add {
                    let (marker, _pos, is_present) = markers_value.get_mut(idx).unwrap();
                    marker.addTo(&leaflet);
                    log::info!("Adding marker {idx} to map");
                    *is_present = true;
                }
                for idx in to_remove {
                    let (marker, _pos, is_present) = markers_value.get_mut(idx).unwrap();

                    marker.remove();
                    log::info!("Removing marker {idx} from map");
                    *is_present = false;
                }

                markers.set(markers_value);
            }
        });

        leaflet_map.on("move", &handler.into_js_value());

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

    let invalidate_cb = Callback::from({
        let leaflet = (*leaflet).clone();
        move |e: MouseEvent| {
            web_sys::console::log_1(&leaflet);
            e.prevent_default();
            leaflet.invalidateSize(true);
        }
    });

    // To render the map, need to create VRef to the map's element
    let node: &Node = &container.clone().into();
    html! {
        <div class={classes!("map-container", props.class.clone())} style={&props.style}>
            {Html::VRef(node.clone())}
            <button onclick={invalidate_cb}>{"InvalidateSize()"}</button>
        </div>
    }
}

fn add_tile_layer(map: &Map) {
    TileLayer::new("http://localhost:3000/_/{z}/{x}/{y}.png", &JsValue::NULL).addTo(map);
}
