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
    let app = Router::new()
        .route("/", get(|| async { "Slippy map tile server!" }))
        .route("/:style/:zoom/:x/:y_png", get(fetch_tile));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[cfg(feature = "online")]
fn get_tile_url(_style: &str, zoom: i32, x: i32, y: i32) -> String {
    // TODO: multiple styles, custom tile server
    format!("https://tile.openstreetmap.org/{zoom}/{x}/{y}.png")
}

async fn fetch_tile(Path((style, zoom, x, y)): Path<(String, i32, i32, String)>) -> Response {
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
    let y: i32 = match y.parse() {
        Ok(y) => y,
        Err(why) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Error parsing last path component into number: {why}"),
            )
                .into_response()
        }
    };

    async fn inner_fetch_tile(style: String, zoom: i32, x: i32, y: i32) -> Result<Vec<u8>, String> {
        let existing_file = tokio::fs::read(format!("tile-cache/{style}/{zoom}/{x}/{y}.png")).await;
        match existing_file {
            Ok(contents) => {
                return Ok(contents);
            }
            Err(why) => {
                #[cfg(not(feature = "online"))]
                {
                    return Err(format!("Could not read tile {style}/{zoom}/{x}/{y}\n{why}\nWill not attempt fetching from web\nEnable 'online' feature for fetching"));
                }

                #[cfg(feature = "online")]
                {
                    drop(why);
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
                                if let Err(why) = tokio::fs::create_dir_all(format!(
                                    "tile-cache/{style}/{zoom}/{x}"
                                ))
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

                                Ok(body.to_vec())
                            }
                        },
                    }
                }
            }
        }
    }

    match inner_fetch_tile(style, zoom, x, y).await {
        Ok(img) => {
            let mut resp = img.into_response();
            resp.headers_mut()
                .insert("Content-Type", HeaderValue::from_static("image/png"));
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
    let mut output = vec![];
    let mut writer = std::io::BufWriter::new(Cursor::new(&mut output));
    image.write_to(&mut writer, ImageOutputFormat::Png).unwrap();

    drop(writer);

    // Export the PNG
    output
}