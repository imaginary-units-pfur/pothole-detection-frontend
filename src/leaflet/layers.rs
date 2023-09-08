use super::MyTileLayer;

#[derive(serde::Serialize)]
struct LayerOptions {
    attribution: String,
}

pub fn get_default_layer() -> MyTileLayer {
    let options = LayerOptions {
        attribution: r#"&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors"#.to_string(),
    };
    let options = serde_wasm_bindgen::to_value(&options).unwrap();
    MyTileLayer::new("http://localhost:3000/_/{s}/{z}/{x}/{y}.png", &options)
}

pub fn get_transport_layer() -> MyTileLayer {
    let options = LayerOptions {
        attribution: r#"&copy; <a href="http://www.thunderforest.com/">Thunderforest</a>, &copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors"#.to_string(),
    };
    let options = serde_wasm_bindgen::to_value(&options).unwrap();
    MyTileLayer::new(
        "http://localhost:3000/transportdark/{s}/{z}/{x}/{y}.png",
        &options,
    )
}

pub fn get_matrix_layer() -> MyTileLayer {
    let options = LayerOptions {
        attribution: r#"'<a href="http://jawg.io" title="Tiles Courtesy of Jawg Maps" target="_blank">&copy; <b>Jawg</b>Maps</a> &copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors"#.to_string(),
    };
    let options = serde_wasm_bindgen::to_value(&options).unwrap();
    MyTileLayer::new("http://localhost:3000/matrix/{s}/{z}/{x}/{y}.png", &options)
}
