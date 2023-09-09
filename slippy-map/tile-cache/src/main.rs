use std::io::Cursor;

use axum::{
    extract::{Path, State},
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use image::{ImageOutputFormat, RgbImage};
#[cfg(feature = "online")]
use tokio::io::AsyncWriteExt;

#[cfg(feature = "online")]
#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
}

#[cfg(not(feature = "online"))]
#[derive(Clone)]
struct AppState {}

impl AppState {
    #[cfg(feature = "online")]
    pub fn new() -> Self {
        AppState {
            client: reqwest::Client::new(),
        }
    }

    #[cfg(not(feature = "online"))]
    pub fn new() -> Self {
        AppState {}
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(|| async { "Slippy map tile server!" }))
        .route("/:style/:idx/:zoom/:x/:y_png", get(fetch_tile))
        .route(
            "/precache-until-zoom/:style/:zoom",
            get(precache_until_zoom),
        )
        .route(
            "/precache-moscow-until-zoom/:style/:zoom",
            get(precache_moscow_until_zoom),
        )
        .with_state(AppState::new());

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .http1_keepalive(true)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
#[cfg(feature = "online")]
async fn precache_until_zoom(
    Path((style, zoom)): Path<(String, u8)>,
    State(state): State<AppState>,
) -> String {
    let mut idx_loop = (0..).map(|i| ["a", "b", "c"][i % 3]);

    let mut errors = 0;
    let mut existing = 0;
    let mut new = 0;

    for tile in slippy_map_tiles::Tile::all_to_zoom(zoom) {
        let result = inner_fetch_tile(
            &state,
            style.to_string(),
            idx_loop.next().unwrap().to_string(),
            tile.zoom(),
            tile.x(),
            tile.y(),
        )
        .await;
        match result {
            Err(e) => {
                tracing::error!("Error: {e}");
                errors += 1;
            }
            Ok(ans) => match ans {
                (_, true) => {
                    existing += 1;
                }
                (_, false) => {
                    new += 1;
                }
            },
        }
    }

    format!("Errors: {errors}, existing tiles: {existing}, new tiles: {new}")
}

#[cfg(not(feature = "online"))]
async fn precache_until_zoom(
    Path(_args): Path<(String, u8)>,
    State(_state): State<AppState>,
) -> &'static str {
    "Compiled without online support, cannot perform any fetch"
}

#[cfg(feature = "online")]
async fn precache_moscow_until_zoom(
    Path((style, zoom)): Path<(String, u8)>,
    State(state): State<AppState>,
) -> String {
    let mut idx_loop = (0..).map(|i| ["a", "b", "c"][i % 3]);

    let mut errors = 0;
    let mut existing = 0;
    let mut new = 0;

    let moscow = slippy_map_tiles::BBox::new(57.0, 36.0, 55.0, 38.0).unwrap();
    for tile in slippy_map_tiles::Tile::all_to_zoom(zoom) {
        if !tile.bbox().overlaps_bbox(&moscow) {
            tracing::debug!("Skip {tile:?}");
            continue;
        }
        let result = inner_fetch_tile(
            &state,
            style.to_string(),
            idx_loop.next().unwrap().to_string(),
            tile.zoom(),
            tile.x(),
            tile.y(),
        )
        .await;
        match result {
            Err(e) => {
                tracing::error!("Error: {e}");
                errors += 1;
            }
            Ok(ans) => match ans {
                (_, true) => {
                    existing += 1;
                }
                (_, false) => {
                    new += 1;
                }
            },
        }
    }

    format!("Errors: {errors}, existing tiles: {existing}, new tiles: {new}")
}

#[cfg(not(feature = "online"))]
async fn precache_moscow_until_zoom(
    Path(_args): Path<(String, u8)>,
    State(_state): State<AppState>,
) -> &'static str {
    "Compiled without online support, cannot perform any fetch"
}

#[cfg(feature = "online")]
fn get_tile_url(style: &str, idx: &str, zoom: u8, x: u32, y: u32) -> String {
    match style {
        "_" => format!("https://tile.openstreetmap.org/{zoom}/{x}/{y}.png"),
        "transportdark" => format!("https://{idx}.tile.thunderforest.com/transport-dark/{zoom}/{x}/{y}.png?apikey=db5ae1f5778a448ca662554581f283c5"),
        "matrix" => format!("https://{idx}.tile.jawg.io/jawg-matrix/{zoom}/{x}/{y}.png?access-token=PyTJUlEU1OPJwCJlW1k0NC8JIt2CALpyuj7uc066O7XbdZCjWEL3WYJIk6dnXtps"),
        _ => panic!("Unknown style: {style}"),
    }
    // TODO: multiple styles, custom tile server
}

#[cfg(feature = "online")]
fn precache_adjacent_tiles(state: &AppState, style: String, idx: String, zoom: u8, x: u32, y: u32) {
    tracing::info!("Precaching tiles for {style}/{zoom}/{x}/{y}");
    // Also download the tiles several levels below the one we have, and all the levels above.
    let this_tile = slippy_map_tiles::Tile::new(zoom, x, y).unwrap();

    #[async_recursion::async_recursion]
    async fn fetch_subtiles(
        this_tile: slippy_map_tiles::Tile,
        state: &AppState,
        style: &str,
        idx: &str,
        levels: u8,
    ) {
        if levels == 0 {
            return;
        }

        if let Some(subtiles) = this_tile.subtiles() {
            for tile in subtiles {
                tokio::spawn({
                    let state = state.clone();
                    let style = style.to_string();
                    let idx = idx.to_string();
                    async move {
                        inner_fetch_tile(&state, style, idx, tile.zoom(), tile.x(), tile.y()).await
                    }
                });

                fetch_subtiles(tile, &state, style, idx, levels - 1).await;
            }
        }
    }

    tokio::spawn({
        let state = state.clone();
        let style = style.to_string();
        let idx = idx.to_string();
        async move { fetch_subtiles(this_tile, &state, &style, &idx, 2).await }
    });

    let mut supertile = this_tile.parent();
    while let Some(tile) = supertile {
        tokio::spawn({
            let state = state.clone();
            let style = style.clone();
            let idx = idx.clone();

            async move { inner_fetch_tile(&state, style, idx, tile.zoom(), tile.x(), tile.y()).await }
        });
        supertile = tile.parent();
    }
}

#[cfg(all(feature = "debug-highlight-fresh", feature = "online"))]
fn mark_fresh(png_data: Vec<u8>) -> (Vec<u8>, bool) {
    let image = image::load_from_memory(&png_data).unwrap();
    let angle: i32 = rand::random();
    let angle = angle % 360;
    let mut image = image::imageops::huerotate(&image, angle);
    image::imageops::invert(&mut image);

    for i in 0..256 {
        for offset in 0..2 {
            image[(offset, i)] = image::Rgba([255, 0, 255, 255]);
            image[(i, offset)] = image::Rgba([255, 0, 255, 255]);
            image[(255 - offset, i)] = image::Rgba([255, 0, 255, 255]);
            image[(i, 255 - offset)] = image::Rgba([255, 0, 255, 255]);
        }
    }

    let mut output: Vec<u8> = vec![];
    let mut writer = std::io::BufWriter::new(Cursor::new(&mut output));
    image.write_to(&mut writer, ImageOutputFormat::Png).unwrap();

    drop(writer);
    (output, false)
}

#[cfg(all(not(feature = "debug-highlight-fresh"), feature = "online"))]
fn mark_fresh(png_data: Vec<u8>) -> (Vec<u8>, bool) {
    (png_data, true)
}

async fn inner_fetch_tile(
    state: &AppState,
    style: String,
    idx: String,
    zoom: u8,
    x: u32,
    y: u32,
) -> Result<(Vec<u8>, bool), String> {
    let existing_file = tokio::fs::read(format!("tile-cache/{style}/{zoom}/{x}/{y}.png")).await;
    match existing_file {
        Ok(contents) => {
            tracing::info!("Tile {style}/{zoom}/{x}/{y} already on disk");
            return Ok((contents, true));
        }
        Err(why) => {
            #[cfg(not(feature = "online"))]
            {
                drop(idx);
                tracing::error!(
                    "Tile {style}/{zoom}/{x}/{y} not already on disk, and built without online"
                );
                return Err(format!("Could not read tile {style}/{zoom}/{x}/{y}\n{why}\nWill not attempt fetching from web\nEnable 'online' feature for fetching"));
            }

            #[cfg(feature = "online")]
            {
                drop(why);
                tracing::info!("Downloading tile {style}/{zoom}/{x}/{y}");
                // Try fetching the tile image from the online map provider
                let resp = state.client.get(get_tile_url(&style, &idx, zoom, x, y)).header("Referer", "http://leaflet-extras.github.io").header("User-Agent","pothole-detection-frontend/0.1, +https://github.com/imaginary-units-pfur/pothole-detection-frontend").send().await;
                match resp {
                    Err(why) => {
                        return Err(format!(
                            "Could not fetch tile {style}/{zoom}/{x}/{y}\n{why}"
                        ))
                    }
                    Ok(resp) => match resp.error_for_status() {
                        Err(why) => {
                            return Err(format!(
                                "Could not fetch tile {style}/{zoom}/{x}/{y}\n{why}"
                            ))
                        }
                        Ok(resp) => {
                            // Download response body
                            let body = match resp.bytes().await {
                                Ok(b) => b,
                                Err(why) => {
                                    return Err(format!(
                                        "Could not fetch tile {style}/{zoom}/{x}/{y}\n{why}"
                                    ))
                                }
                            };

                            // Store this to a file
                            if let Err(why) =
                                tokio::fs::create_dir_all(format!("tile-cache/{style}/{zoom}/{x}"))
                                    .await
                            {
                                return Err(format!(
                                    "Could not save tile {style}/{zoom}/{x}/{y}\n{why}"
                                ));
                            }
                            let mut file = match tokio::fs::File::create(format!(
                                "tile-cache/{style}/{zoom}/{x}/{y}.png"
                            ))
                            .await
                            {
                                Ok(f) => f,
                                Err(why) => {
                                    return Err(format!(
                                        "Could not save tile {style}/{zoom}/{x}/{y}\n{why}"
                                    ));
                                }
                            };

                            if let Err(why) = file.write_all(&body).await {
                                return Err(format!(
                                    "Could not save tile {style}/{zoom}/{x}/{y}\n{why}"
                                ));
                            };

                            Ok(mark_fresh(body.to_vec()))
                        }
                    },
                }
            }
        }
    }
}

async fn fetch_tile(
    Path((style, idx, zoom, x, y)): Path<(String, String, u8, u32, String)>,
    State(state): State<AppState>,
) -> Response {
    let y = y.split(".").nth(0);
    let y = match y {
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "Last path component must have a number before a dot, like: `123.png`",
            )
                .into_response();
        }
        Some(y) => y,
    };
    let y: u32 = match y.parse() {
        Ok(y) => y,
        Err(why) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Error parsing last path component into number: {why}"),
            )
                .into_response()
        }
    };

    match inner_fetch_tile(&state, style, idx.to_string(), zoom, x, y).await {
        Ok((img, is_cacheable)) => {
            let mut resp = img.into_response();
            resp.headers_mut()
                .insert("Content-Type", HeaderValue::from_static("image/png"));
            if is_cacheable {
                resp.headers_mut().insert(
                    "Cache-Control",
                    HeaderValue::from_static("max-age=604800, public, immutable"),
                );
            } else {
                resp.headers_mut().insert(
                    "Cache-Control",
                    HeaderValue::from_static("no-cache, no-store"),
                );
            }
            #[cfg(feature = "online")]
            {
                let state = state.clone();

                precache_adjacent_tiles(&state, "_".to_string(), idx.to_string(), zoom, x, y);
                precache_adjacent_tiles(
                    &state,
                    "transportdark".to_string(),
                    idx.to_string(),
                    zoom,
                    x,
                    y,
                );
                precache_adjacent_tiles(&state, "matrix".to_string(), idx.to_string(), zoom, x, y);
            }

            resp
        }
        Err(why) => {
            let mut resp = render_error_image(&why).into_response();
            resp.headers_mut()
                .insert("Content-Type", HeaderValue::from_static("image/png"));
            resp
        }
    }
}

fn render_error_image(text: &str) -> Vec<u8> {
    // Start by creating an image, fill it with a magenta-black pattern
    let mut image = RgbImage::from_fn(256, 256, |x, y| {
        if (x / 16 + y / 16) % 2 == 0 {
            image::Rgb([127, 0, 127])
        } else {
            image::Rgb([0, 0, 0])
        }
    });

    // Draw a bright magenta border
    for i in 0..256 {
        for offset in 0..4 {
            image[(offset, i)] = image::Rgb([255, 0, 255]);
            image[(i, offset)] = image::Rgb([255, 0, 255]);
            image[(255 - offset, i)] = image::Rgb([255, 0, 255]);
            image[(i, 255 - offset)] = image::Rgb([255, 0, 255]);
        }
    }

    // Draw some text on it
    // Font: https://dejavu-fonts.github.io/Download.html
    let font = include_bytes!("../DejaVuSans-Bold.ttf");
    let font = rusttype::Font::try_from_bytes(font).unwrap();
    for (i, line) in textwrap::wrap(text, 20).iter().enumerate() {
        image = imageproc::drawing::draw_text(
            &image,
            image::Rgb([255, 255, 255]),
            0,
            24 * i as i32,
            rusttype::Scale { x: 24.0, y: 24.0 },
            &font,
            line,
        );
    }

    // Save the image as a PNG
    let mut output: Vec<u8> = vec![];
    let mut writer = std::io::BufWriter::new(Cursor::new(&mut output));
    image.write_to(&mut writer, ImageOutputFormat::Png).unwrap();

    drop(writer);

    // Export the PNG
    output
}
