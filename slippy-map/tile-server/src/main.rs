use std::{io::Cursor, ops::Index};

use axum::{
    extract::Path,
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use image::{ImageOutputFormat, RgbImage};

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
        return Err(format!("Tile: {style}/{zoom}/{x}/{y}\nNewline test"));
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
    for (i, line) in text.split("\n").enumerate() {
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
