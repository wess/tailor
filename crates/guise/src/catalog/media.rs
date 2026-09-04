//! Images and the placeholder that stands in for one.

use tailor_model::props::{enums, float, size, text, Emit, PropValue};
use tailor_model::tokens::SizeToken;

use tailor_model::catalog::{ComponentSpec, Ctor};
use tailor_model::comp;

pub static SPECS: &[ComponentSpec] = &[comp!(
    "image", "Image", "Image", Media, "image",
    "An image from a path or a URL.",
    Ctor::Arg("source"),
    props: &[
        tailor_model::props::hinted(
            text("source", "Source", Emit::None),
            "a file path or an http(s) URL",
        ),
        float("width", "Width", Emit::Method("width"), || PropValue::Float(160.0)),
        float("height", "Height", Emit::Method("height"), || PropValue::Float(120.0)),
        size("radius", "Radius", Emit::Method("radius"), SizeToken::Md),
        enums("fit", "Fit", Emit::Method("fit"), "ObjectFit",
            &["fill", "contain", "cover", "none"], || PropValue::Choice("cover".into())),
    ],
    required: &[("source", "Point it at a file path or a URL.")],
)];
