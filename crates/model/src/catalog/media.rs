//! Images and the placeholder that stands in for one.

use crate::props::{enums, float, size, text, Emit, PropValue};
use crate::tokens::SizeToken;

use super::spec::{ComponentSpec, Ctor};

pub static SPECS: &[ComponentSpec] = &[comp!(
    "image", "Image", "Image", Media, "image",
    "An image from a path or a URL.",
    Ctor::Arg("source"),
    props: &[
        crate::props::hinted(
            text("source", "Source", Emit::None),
            "a file path or an http(s) URL",
        ),
        float("width", "Width", Emit::Method("width"), || PropValue::Float(160.0)),
        float("height", "Height", Emit::Method("height"), || PropValue::Float(120.0)),
        size("radius", "Radius", Emit::Method("radius"), SizeToken::Md),
        enums("fit", "Fit", Emit::Method("fit"), "ObjectFit",
            &["fill", "contain", "cover", "none"], || PropValue::Choice("cover".into())),
    ],
)];
