//! The canvas-painted charts.
//!
//! Every series is a `Numbers` prop, so a chart on the canvas draws real data
//! and the generated code carries the same literal. Swapping that literal for
//! your own vector is the one edit a chart needs after export.

use crate::props::{boolean, color, float, items, numbers, Emit, PropValue};
use crate::tokens::ColorToken;

use super::spec::{ComponentSpec, Ctor};

fn series() -> PropValue {
    PropValue::Numbers(vec![12.0, 19.0, 8.0, 24.0, 16.0, 28.0, 21.0])
}

fn labels() -> PropValue {
    PropValue::Items(vec![
        "Mon".into(),
        "Tue".into(),
        "Wed".into(),
        "Thu".into(),
        "Fri".into(),
        "Sat".into(),
        "Sun".into(),
    ])
}

pub static SPECS: &[ComponentSpec] = &[
    comp!(
        "sparkline", "Sparkline", "Sparkline", Charts, "activity",
        "A bare trend line.",
        Ctor::Arg("values"),
        props: &[
            numbers("values", "Values", Emit::None, series),
            color("color", "Color", Emit::Method("color"), ColorToken::Blue),
            float("stroke", "Stroke", Emit::Method("stroke"), || PropValue::Float(2.0)),
            boolean("fill", "Fill", Emit::Flag("fill"), false),
            float("width", "Width", Emit::Method("width"), || PropValue::Float(120.0)),
            float("height", "Height", Emit::Method("height"), || PropValue::Float(32.0)),
        ],
    ),
    comp!(
        "linechart", "Line chart", "LineChart", Charts, "chart-line",
        "A line chart with optional axes and hover.",
        Ctor::Arg("values"),
        props: &[
            numbers("values", "Values", Emit::None, series),
            items("labels", "Labels", Emit::Method("labels"), labels),
            color("color", "Color", Emit::Method("color"), ColorToken::Blue),
            float("stroke", "Stroke", Emit::Method("stroke"), || PropValue::Float(2.0)),
            boolean("fill", "Fill", Emit::Flag("fill"), false),
            boolean("axis", "Axes", Emit::Flag("axis"), true),
            boolean("hover", "Hover", Emit::Flag("hover"), true),
            float("width", "Width", Emit::Method("width"), || PropValue::Float(360.0)),
            float("height", "Height", Emit::Method("height"), || PropValue::Float(180.0)),
        ],
    ),
    comp!(
        "areachart", "Area chart", "AreaChart", Charts, "chart-area",
        "A filled line chart.",
        Ctor::Arg("values"),
        props: &[
            numbers("values", "Values", Emit::None, series),
            items("labels", "Labels", Emit::Method("labels"), labels),
            boolean("axis", "Axes", Emit::Flag("axis"), true),
            boolean("overlaid", "Overlaid", Emit::Flag("overlaid"), false),
            float("width", "Width", Emit::Method("width"), || PropValue::Float(360.0)),
            float("height", "Height", Emit::Method("height"), || PropValue::Float(180.0)),
        ],
    ),
    comp!(
        "barchart", "Bar chart", "BarChart", Charts, "chart-column",
        "Vertical bars, optionally labelled.",
        Ctor::Arg("values"),
        props: &[
            numbers("values", "Values", Emit::None, series),
            items("labels", "Labels", Emit::Custom, labels),
            color("color", "Color", Emit::Method("color"), ColorToken::Blue),
            float("gap", "Gap", Emit::Method("gap"), || PropValue::Float(6.0)),
            boolean("axis", "Axes", Emit::Flag("axis"), true),
            boolean("hover", "Hover", Emit::Flag("hover"), true),
            float("width", "Width", Emit::Method("width"), || PropValue::Float(360.0)),
            float("height", "Height", Emit::Method("height"), || PropValue::Float(180.0)),
        ],
    ),
    comp!(
        "piechart", "Pie chart", "PieChart", Charts, "chart-pie",
        "A pie or donut.",
        Ctor::Arg("values"),
        props: &[
            numbers("values", "Values", Emit::None, || {
                PropValue::Numbers(vec![40.0, 25.0, 20.0, 15.0])
            }),
            float("size", "Diameter", Emit::Method("size"), || PropValue::Float(160.0)),
            float("donut", "Donut hole", Emit::Method("donut"), || PropValue::Float(0.0)),
        ],
    ),
    comp!(
        "scatterchart", "Scatter chart", "ScatterChart", Charts, "chart-scatter",
        "Points on two axes. Values pair up x, y, x, y.",
        Ctor::Special,
        props: &[
            crate::props::hinted(
                numbers("values", "Values", Emit::None, || {
                    PropValue::Numbers(vec![1.0, 4.0, 2.0, 7.0, 3.0, 3.0, 4.0, 9.0, 5.0, 6.0])
                }),
                "alternating x and y",
            ),
            boolean("hover", "Hover", Emit::Flag("hover"), true),
            float("width", "Width", Emit::Method("width"), || PropValue::Float(360.0)),
            float("height", "Height", Emit::Method("height"), || PropValue::Float(180.0)),
        ],
    ),
];
