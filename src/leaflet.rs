mod icons;

use std::{collections::HashMap, rc::Rc};

use gloo_utils::document;
use leaflet::{Icon, LatLng, Map, Marker, TileLayer};
use rand::{Rng, SeedableRng};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{HtmlElement, Node};
use yew::prelude::*;
use yew_hooks::{use_async, use_is_first_mount};

#[derive(PartialEq, Properties, Clone)]
pub struct Props {
    #[prop_or_default]
    pub class: Classes,
    #[prop_or_default]
    pub style: AttrValue,
}

#[function_component]
pub fn MapComponent(props: &Props) -> Html {
    let leaflet_box = use_state(|| Option::<Rc<Map>>::None);
    let container_box = use_state(|| None);
    let markers: yew_hooks::UseListHandle<(Marker, LatLng, bool)> = yew_hooks::use_list(vec![]);

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
        async move {
            if let Some(leaflet) = leaflet.as_ref() {
                log::info!("Starting recalculating markers for area!");
                let bounds = leaflet.getBounds();
                let new_markers = {
                    // TODO: real HTTP request
                    let _response = gloo_net::http::Request::get("https://httpbin.org/anything")
                        .send()
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
                        #[derive(serde::Serialize)]
                        #[allow(non_snake_case)]
                        struct IconOptions {
                            iconUrl: String,
                            iconSize: Vec<u32>,
                        }
                        let options = IconOptions {
                            iconUrl: "/marker.png".to_string(),
                            iconSize: vec![32, 32],
                        };

                        let options = serde_wasm_bindgen::to_value(&options).unwrap();
                        web_sys::console::log_1(&options);
                        let icon = Icon::new(&options);
                        for _ in 0..100 {
                            let pos = LatLng::new(
                                rng.gen_range(lat_range.clone()),
                                rng.gen_range(lng_range.clone()),
                            );
                            let marker = Marker::new(&pos);

                            marker.setIcon(&icon);
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

    let (_leaflet, container) = if use_is_first_mount() {
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

    // To render the map, need to create VRef to the map's element
    let node: &Node = &container.clone().into();
    let loading = perform_bbox_fetch.loading;
    html! {
        <div class={classes!("map-container", props.class.clone())}>
            <div class={classes!("card", loading.then_some("placeholder-wave"))}>
                <div class={classes!("card-body", loading.then_some("text-warning placeholder"))} style={&props.style}>
                    {Html::VRef(node.clone())}
                </div>
            </div>
        </div>
    }
}

fn add_tile_layer(map: &Map) {
    TileLayer::new("http://localhost:3000/_/{z}/{x}/{y}.png", &JsValue::NULL).addTo(map);
}
