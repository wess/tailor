//! The toast stack, and the three severities Tailor uses it for.

use gpui::prelude::*;
use gpui::{App, Entity};
use guise::prelude::*;

#[derive(Clone)]
pub struct Toasts {
    stack: Entity<ToastStack>,
}

impl Toasts {
    pub fn new(cx: &mut App) -> Self {
        Toasts {
            stack: cx.new(|_| ToastStack::new()),
        }
    }

    pub fn stack(&self) -> Entity<ToastStack> {
        self.stack.clone()
    }

    pub fn info(&self, message: impl Into<gpui::SharedString>, cx: &mut App) {
        let message = message.into();
        self.stack.update(cx, |stack, cx| {
            stack.push(message, cx);
        });
    }

    pub fn done(&self, message: impl Into<gpui::SharedString>, cx: &mut App) {
        self.titled("Done", message, ColorName::Green, cx);
    }

    pub fn failed(&self, message: impl Into<gpui::SharedString>, cx: &mut App) {
        self.titled("Failed", message, ColorName::Red, cx);
    }

    pub fn titled(
        &self,
        title: impl Into<gpui::SharedString>,
        message: impl Into<gpui::SharedString>,
        color: ColorName,
        cx: &mut App,
    ) {
        let (title, message) = (title.into(), message.into());
        self.stack.update(cx, |stack, cx| {
            stack.push_titled(title, message, color, cx);
        });
    }
}
