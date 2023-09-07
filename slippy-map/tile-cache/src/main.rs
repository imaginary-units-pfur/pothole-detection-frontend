use std::io::Cursor;

use axum::{
    extract::Path,
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use image::{ImageOutputFormat, RgbImage};
#[cfg(feature = "online")]
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(|| async { "Slippy map tile server!" }))
        .route("/:style/:zoom/:x/:y_png", get(fetch_tile));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[cfg(feature = "online")]
fn get_tile_url(_style: &str, zoom: u8, x: u32, y: u32) -> String {
    // TODO: multiple styles, custom tile server
    format!("https://tile.openstreetmap.org/{zoom}/{x}/{y}.png")
}

#[cfg(feature = "online")]
async fn precache_adjacent_tiles(style: String, zoom: u8, x: u32, y: u32) {
    tracing::info!("Precaching tiles for {style}/{zoom}/{x}/{y}");
    // Also download the tiles one level below the one we have, and all the levels above.
    let this_tile = slippy_map_tiles::Tile::new(zoom, x, y).unwrap();
    if let Some(subtiles) = this_tile.subtiles() {
        for tile in subtiles {
            tokio::spawn(inner_fetch_tile(
                style.to_string(),
                tile.zoom(),
                tile.x(),
                tile.y(),
            ));
        }
    }

    let mut supertile = this_tile.parent();
    while let Some(tile) = supertile {
        tokio::spawn(inner_fetch_tile(
            style.to_string(),
            tile.zoom(),
            tile.x(),
            tile.y(),
        ));
        supertile = tile.parent();
    }
}

#[cfg(all(feature = "debug-highlight-fresh", feature = "online"))]
fn mark_fresh(png_data: Vec<u8>) -> Vec<u8> {
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
    output
}

#[cfg(all(not(feature = "debug-highlight-fresh"), feature = "online"))]
fn mark_fresh(png_data: Vec<u8>) -> Vec<u8> {
    png_data
}

async fn inner_fetch_tile(style: String, zoom: u8, x: u32, y: u32) -> Result<Vec<u8>, String> {
    let existing_file = tokio::fs::read(format!("tile-cache/{style}/{zoom}/{x}/{y}.png")).await;
    match existing_file {
        Ok(contents) => {
            tracing::info!("Tile {style}/{zoom}/{x}/{y} already on disk");
            return Ok(contents);
        }
        Err(why) => {
            #[cfg(not(feature = "online"))]
            {
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
                let client = reqwest::Client::new();
                let resp = client.get(get_tile_url(&style, zoom, x, y)).header("User-Agent","pothole-detection-frontend/0.1, +https://github.com/imaginary-units-pfur/pothole-detection-frontend").send().await;
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

async fn fetch_tile(Path((style, zoom, x, y)): Path<(String, u8, u32, String)>) -> Response {
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

    match inner_fetch_tile(style.to_string(), zoom, x, y).await {
        Ok(img) => {
            let mut resp = img.into_response();
            resp.headers_mut()
                .insert("Content-Type", HeaderValue::from_static("image/png"));
            #[cfg(feature = "online")]
            tokio::spawn(precache_adjacent_tiles(style, zoom, x, y));

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
