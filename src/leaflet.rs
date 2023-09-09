mod icons;
mod layers;

use std::{rc::Rc, str::FromStr};

use gloo_utils::document;
use leaflet::{LatLng, Map, Marker};
use rand::{Rng, SeedableRng};
use reqwest::{Method, Request};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{HtmlElement, Node};
use yew::prelude::*;
use yew_hooks::{use_async, use_is_first_mount};

use crate::{
    leaflet::{
        icons::IconGenerator,
        layers::{get_default_layer, get_matrix_layer, get_transport_layer},
    },
    point_display::PointDisplay,
};

#[derive(PartialEq, Properties, Clone)]
pub struct Props {
    #[prop_or_default]
    pub class: Classes,
    #[prop_or_default]
    pub style: AttrValue,
}

#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[derive(Debug)]
    pub type MyTileLayer;

    #[wasm_bindgen(constructor, js_namespace = L, js_class="TileLayer")]
    pub fn new(url_template: &str, options: &JsValue) -> MyTileLayer;

    #[wasm_bindgen(method, js_class = "TileLayer")]
    pub fn addTo(this: &MyTileLayer, map: &Map);

    #[wasm_bindgen(method, js_class = "TileLayer")]
    pub fn remove(this: &MyTileLayer);
}

#[function_component]
pub fn MapComponent(props: &Props) -> Html {
    let leaflet_box = use_state(|| Option::<Rc<Map>>::None);
    let container_box = use_state(|| None);
    let current_layer = use_state(|| None);
    enum LayerStyle {
        Default,
        TransportDark,
        Matrix,
    }
    let current_layer_style = use_state(|| LayerStyle::Default);

    let markers: yew_hooks::UseListHandle<(Marker, LatLng, bool)> = yew_hooks::use_list(vec![]);

    let clicked_point_info = use_state(|| Option::<usize>::None);

    // For ensuring that the map container is always the correct size,
    // as well as for invalidating its initial size of zero (before the element is drawn to the screen)
    let _leaflet_invalidate_size = yew_hooks::use_interval(
        {
            let leaflet_box = leaflet_box.clone();
            move || {
                if let Some(leaflet) = leaflet_box.as_ref() {
                    leaflet.invalidateSize(true);
                }
            }
        },
        200,
    );

    let perform_bbox_fetch = use_async({
        let leaflet = leaflet_box.clone();
        let markers = markers.clone();
        let clicked_point_info = clicked_point_info.clone();
        async move {
            if let Some(leaflet) = leaflet.as_ref() {
                log::info!("Starting recalculating markers for area!");
                let bounds = leaflet.getBounds();
                let new_markers = {
                    // TODO: real HTTP request
                    let client = reqwest::Client::new();
                    let _response = client
                        .execute(Request::new(
                            Method::GET,
                            reqwest::Url::from_str("https://httpbin.org/anything").unwrap(),
                        ))
                        .await;

                    log::info!("Starting building markers");
                    let mut markers = vec![];
                    let ne = bounds.getNorthEast();
                    let sw = bounds.getSouthWest();
                    let ne = (ne.lat(), ne.lng());
                    let sw = (sw.lat(), sw.lng());
                    let lat_range = if ne.0 > sw.0 { sw.0..ne.0 } else { ne.0..sw.0 };
                    let lng_range = if ne.1 > sw.1 { sw.1..ne.1 } else { ne.1..sw.1 };
                    let bound_as_u64: u64 = unsafe { std::mem::transmute(ne.0 + sw.1) };
                    let mut rng = rand::rngs::StdRng::seed_from_u64(bound_as_u64);
                    if ne.0 == sw.0 || ne.1 == sw.1 {
                        log::error!("Not generating data as the map has zero size");
                        markers
                    } else {
                        let mut gen = IconGenerator::default();
                        let mut icons = vec![];
                        icons.push(gen.bump());
                        icons.push(gen.crack());
                        icons.push(gen.hole());
                        icons.push(gen.patch());

                        for i in 0..100 {
                            let pos = LatLng::new(
                                rng.gen_range(lat_range.clone()),
                                rng.gen_range(lng_range.clone()),
                            );
                            let marker = Marker::new(&pos);

                            marker.setIcon(&icons[i % icons.len()]);
                            let click_handler: Closure<dyn FnMut(leaflet::MouseEvent) -> ()> =
                                Closure::new({
                                    let clicked_point_id = clicked_point_info.clone();
                                    move |_e| {
                                        clicked_point_id.set(Some(i));
                                    }
                                });

                            marker.on("click", &click_handler.into_js_value());
                            markers.push((marker, pos, true));
                        }
                        markers
                    }
                };

                for (old_marker, _pos, is_visible) in markers.current().iter() {
                    if *is_visible {
                        old_marker.remove();
                    }
                }

                for (new_marker, _pos, _is_visible) in new_markers.iter() {
                    new_marker.addTo(&leaflet);
                }

                markers.set(new_markers);
                log::info!("Markers are built!");
            }

            Ok::<(), ()>(())
        }
    });

    let (leaflet, container) = if use_is_first_mount() {
        // Initialize the target HTML element
        let container = document().create_element("div").unwrap();
        let container: HtmlElement = container.dyn_into().unwrap();
        container.set_class_name("map");
        let leaflet_map = Map::new_with_element(&container, &JsValue::NULL);

        leaflet_map.setView(&LatLng::new(55.34, 37.78), 11.0);

        let layer = get_default_layer();
        layer.addTo(&leaflet_map);
        current_layer_style.set(LayerStyle::Default);
        current_layer.set(Some(layer));

        container_box.set(Some(container.clone()));
        let leaflet_map = Rc::new(leaflet_map);

        let move_handler: Closure<dyn FnMut(leaflet::MouseEvent) -> ()> = Closure::new({
            let leaflet = leaflet_map.clone();
            let markers = markers.clone();

            move |_e| {
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

        leaflet_map.on("move", &move_handler.into_js_value());

        let move_finish_handler: Closure<dyn FnMut(leaflet::MouseEvent) -> ()> = Closure::new({
            let perform_bbox_fetch = perform_bbox_fetch.clone();
            move |_e| {
                log::info!("Map drag is complete, querying for new marker state");
                perform_bbox_fetch.run()
            }
        });

        leaflet_map.on("moveend", &move_finish_handler.into_js_value());

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

    let set_default_style_cb = {
        let leaflet = leaflet.clone();
        let current_layer = current_layer.clone();
        let current_layer_style = current_layer_style.clone();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            let new_layer = get_default_layer();
            let old_layer = current_layer.as_ref().unwrap(); // current_layer is always set when replaced
            old_layer.remove();
            new_layer.addTo(&leaflet);
            current_layer.set(Some(new_layer));
            current_layer_style.set(LayerStyle::Default);
        })
    };
    let set_transport_style_cb: Callback<MouseEvent> = {
        let leaflet = leaflet.clone();
        let current_layer = current_layer.clone();
        let current_layer_style = current_layer_style.clone();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            let new_layer = get_transport_layer();
            let old_layer = current_layer.as_ref().unwrap(); // current_layer is always set when replaced
            old_layer.remove();
            new_layer.addTo(&leaflet);
            current_layer.set(Some(new_layer));
            current_layer_style.set(LayerStyle::TransportDark);
        })
    };

    let set_matrix_style_cb: Callback<MouseEvent> = {
        let leaflet = leaflet.clone();
        let current_layer = current_layer.clone();
        let current_layer_style = current_layer_style.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            let new_layer = get_matrix_layer();
            let old_layer = current_layer.as_ref().unwrap(); // current_layer is always set when replaced
            old_layer.remove();
            new_layer.addTo(&leaflet);
            current_layer.set(Some(new_layer));
            current_layer_style.set(LayerStyle::Matrix);
        })
    };

    let map_style_choice = html! {
        <div class="btn-group" role="group">
            <button class={classes!("btn", if matches!(*current_layer_style, LayerStyle::Default) {"btn-primary"} else {"btn-outline-primary"})} onclick={set_default_style_cb}>
                {"Default"}
            </button>
            <button class={classes!("btn", if matches!(*current_layer_style, LayerStyle::TransportDark) {"btn-primary"} else {"btn-outline-primary"})} onclick={set_transport_style_cb}>
                {"TransportDark"}
            </button>
            <button class={classes!("btn", if matches!(*current_layer_style, LayerStyle::Matrix) {"btn-primary"} else {"btn-outline-primary"})} onclick={set_matrix_style_cb}>
                {"Matrix"}
            </button>
        </div>
    };

    let clear_clicked_cb = Callback::from({
        let clicked_point_id = clicked_point_info.clone();
        move |_| {
            clicked_point_id.set(None);
        }
    });

    // To render the map, need to create VRef to the map's element
    let node: &Node = &container.clone().into();
    let loading = perform_bbox_fetch.loading;
    html! {
        <div class={classes!("map-container", props.class.clone())}>
            <div class="map-and-pointview">

                // Main map widget inside a Card component
                <div class={classes!("card", "map-card", "mapview", loading.then_some("placeholder-wave"))}>
                    <div class={classes!("card-body", "map-card-body", loading.then_some("text-warning placeholder"))} style={&props.style}>
                        {Html::VRef(node.clone())}
                    </div>
                    {map_style_choice}
                </div>

                // Side element containing info about clicked points

                <PointDisplay leaflet={leaflet.clone()} clicked_point_info={*clicked_point_info} {clear_clicked_cb} />
            </div>
        </div>
    }
}
