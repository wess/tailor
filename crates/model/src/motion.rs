//! The entrance a node plays when it appears.
//!
//! A designer's version of `guise::anim`: not the whole keyframe model, but
//! the four decisions that matter for a screen — what the motion is, how it
//! eases, how long it takes, and whether a container's children come in one
//! after another. Everything here maps to one `Motion` in generated code and
//! to the same `Motion` on the canvas, so what you preview is what ships.
//!
//! Stagger is the one rule worth stating twice: a container with a stagger
//! does not animate *itself*, it animates each of its children with an
//! offset. That is what "stagger" means everywhere else, and it keeps a
//! single node from having two animations to reason about.

use crate::tokens::{EaseToken, EnterToken, LoopToken};
use serde::{Deserialize, Serialize};

fn is_false(v: &bool) -> bool {
    !*v
}

fn is_zero(v: &f32) -> bool {
    *v == 0.0
}

fn default_duration() -> f32 {
    260.0
}

fn is_default_duration(v: &f32) -> bool {
    *v == default_duration()
}

fn default_distance() -> f32 {
    8.0
}

fn is_default_distance(v: &f32) -> bool {
    *v == default_distance()
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

/// What a node does when it appears. `enter: None` is "nothing", and is the
/// default for every node — motion is opt-in, one node at a time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MotionProps {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enter: Option<EnterToken>,
    #[serde(skip_serializing_if = "is_default")]
    pub ease: EaseToken,
    /// Milliseconds the motion takes.
    #[serde(skip_serializing_if = "is_default_duration")]
    pub duration: f32,
    /// Milliseconds before it starts.
    #[serde(skip_serializing_if = "is_zero")]
    pub delay: f32,
    /// How far a slide travels, in px. Ignored by `Fade`.
    #[serde(skip_serializing_if = "is_default_distance")]
    pub distance: f32,
    /// Milliseconds between children. Non-zero moves the animation off this
    /// node and onto each of its children, one offset per index.
    #[serde(skip_serializing_if = "is_zero")]
    pub stagger: f32,
    #[serde(skip_serializing_if = "is_default")]
    pub repeat: LoopToken,
    /// Play every other pass backwards — only meaningful when it repeats.
    #[serde(skip_serializing_if = "is_false")]
    pub alternate: bool,
}

impl Default for MotionProps {
    fn default() -> Self {
        MotionProps {
            enter: None,
            ease: EaseToken::default(),
            duration: default_duration(),
            delay: 0.0,
            distance: default_distance(),
            stagger: 0.0,
            repeat: LoopToken::default(),
            alternate: false,
        }
    }
}

impl MotionProps {
    pub fn is_default(&self) -> bool {
        *self == MotionProps::default()
    }

    /// Whether this node carries an animation at all.
    pub fn is_off(&self) -> bool {
        self.enter.is_none()
    }

    /// Whether the animation belongs to this node's children instead of to
    /// this node.
    pub fn staggers(&self) -> bool {
        self.enter.is_some() && self.stagger > 0.0
    }

    /// JSON cannot write an infinity or a NaN — serde turns them into `null`,
    /// which then fails to load — so they have to be caught before a save.
    pub fn has_non_finite(&self) -> bool {
        !self.duration.is_finite()
            || !self.delay.is_finite()
            || !self.distance.is_finite()
            || !self.stagger.is_finite()
    }

    /// This node's own animation, if it has one. A staggering container has
    /// none — its children do.
    pub fn own(&self) -> Option<Resolved> {
        let enter = self.enter?;
        if self.stagger > 0.0 {
            return None;
        }
        Some(self.resolved(enter, self.delay))
    }

    /// The animation this node hands to the child at `index`.
    pub fn for_child(&self, index: usize) -> Option<Resolved> {
        let enter = self.enter?;
        if self.stagger <= 0.0 {
            return None;
        }
        Some(self.resolved(enter, self.delay + self.stagger * index as f32))
    }

    fn resolved(&self, enter: EnterToken, delay: f32) -> Resolved {
        Resolved {
            enter,
            ease: self.ease,
            duration: self.duration.max(0.0),
            delay: delay.max(0.0),
            distance: self.distance,
            repeat: self.repeat,
            alternate: self.alternate,
        }
    }
}

/// One node's animation, with the stagger already folded into the delay.
/// This is what the canvas plays and what the generator prints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolved {
    pub enter: EnterToken,
    pub ease: EaseToken,
    pub duration: f32,
    pub delay: f32,
    pub distance: f32,
    pub repeat: LoopToken,
    pub alternate: bool,
}

impl Resolved {
    /// How long from the moment the screen appears until this node has
    /// settled. Endless motions never do, and report their first pass.
    pub fn settles_at(self) -> f32 {
        self.delay + self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_is_off_by_default_and_writes_nothing() {
        let motion = MotionProps::default();
        assert!(motion.is_off());
        assert!(motion.is_default());
        assert_eq!(serde_json::to_string(&motion).unwrap(), "{}");
    }

    #[test]
    fn a_plain_motion_belongs_to_the_node() {
        let motion = MotionProps {
            enter: Some(EnterToken::SlideUp),
            delay: 40.0,
            ..Default::default()
        };
        let own = motion.own().unwrap();
        assert_eq!(own.enter, EnterToken::SlideUp);
        assert_eq!(own.delay, 40.0);
        assert_eq!(motion.for_child(0), None);
    }

    #[test]
    fn a_stagger_moves_the_motion_onto_the_children() {
        let motion = MotionProps {
            enter: Some(EnterToken::Fade),
            delay: 100.0,
            stagger: 60.0,
            ..Default::default()
        };
        assert!(motion.staggers());
        assert_eq!(motion.own(), None, "the container itself stays put");
        assert_eq!(motion.for_child(0).unwrap().delay, 100.0);
        assert_eq!(motion.for_child(3).unwrap().delay, 280.0);
    }

    #[test]
    fn a_node_with_no_enter_hands_out_nothing() {
        let motion = MotionProps {
            stagger: 60.0,
            ..Default::default()
        };
        assert_eq!(motion.own(), None);
        assert_eq!(motion.for_child(2), None);
        assert!(!motion.staggers());
    }

    #[test]
    fn non_finite_numbers_are_caught_before_a_save() {
        assert!(!MotionProps::default().has_non_finite());
        let motion = MotionProps {
            duration: f32::INFINITY,
            ..Default::default()
        };
        assert!(motion.has_non_finite());
    }

    #[test]
    fn round_trips_through_json() {
        let motion = MotionProps {
            enter: Some(EnterToken::SlideLeft),
            ease: EaseToken::OutBack,
            duration: 400.0,
            delay: 20.0,
            distance: 24.0,
            stagger: 50.0,
            repeat: LoopToken::Forever,
            alternate: true,
        };
        let json = serde_json::to_string(&motion).unwrap();
        assert_eq!(serde_json::from_str::<MotionProps>(&json).unwrap(), motion);
    }
}
