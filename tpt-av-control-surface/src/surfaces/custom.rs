//! Custom surface support: build a [`ControlSurface`] from closures or a
//! declarative mapping, without writing a new trait implementation.

use crate::feedback::Feedback;
use crate::surface::{ControlEvent, ControlSurface};
use tpt_av_control_utils::ControlError;

/// Type of the event-pumping closure.
pub type EventFn = Box<dyn FnMut() -> Result<Option<ControlEvent>, ControlError> + Send>;
/// Type of the feedback closure.
pub type FeedbackFn = Box<dyn FnMut(&Feedback) -> Result<(), ControlError> + Send>;
/// Type of the init closure.
pub type InitFn = Box<dyn FnMut() -> Result<(), ControlError> + Send>;

/// A user-defined control surface assembled from closures.
///
/// ```
/// use tpt_av_control_surface::{ControlEvent, ControlSurface, CustomSurface, Feedback};
///
/// let mut surface = CustomSurface::builder("my-panel")
///     .on_read(|| Ok(Some(ControlEvent::ButtonPress { button: 1 })))
///     .on_feedback(|feedback| {
///         match feedback {
///             Feedback::Led { .. } => println!("led!"),
///             _ => {}
///         }
///         Ok(())
///     })
///     .build();
/// assert!(surface.init().is_ok());
/// assert_eq!(
///     surface.read_event().unwrap(),
///     Some(ControlEvent::ButtonPress { button: 1 })
/// );
/// ```
pub struct CustomSurface {
    name: String,
    on_init: Option<InitFn>,
    on_read: Option<EventFn>,
    on_feedback: Option<FeedbackFn>,
}

/// Builder for [`CustomSurface`].
pub struct CustomSurfaceBuilder {
    name: String,
    on_init: Option<InitFn>,
    on_read: Option<EventFn>,
    on_feedback: Option<FeedbackFn>,
}

impl CustomSurface {
    /// Starts building a custom surface.
    pub fn builder(name: impl Into<String>) -> CustomSurfaceBuilder {
        CustomSurfaceBuilder {
            name: name.into(),
            on_init: None,
            on_read: None,
            on_feedback: None,
        }
    }
}

impl CustomSurfaceBuilder {
    /// Sets the init closure.
    pub fn on_init(mut self, f: impl FnMut() -> Result<(), ControlError> + Send + 'static) -> Self {
        self.on_init = Some(Box::new(f));
        self
    }

    /// Sets the event-pumping closure.
    pub fn on_read(
        mut self,
        f: impl FnMut() -> Result<Option<ControlEvent>, ControlError> + Send + 'static,
    ) -> Self {
        self.on_read = Some(Box::new(f));
        self
    }

    /// Sets the feedback closure.
    pub fn on_feedback(
        mut self,
        f: impl FnMut(&Feedback) -> Result<(), ControlError> + Send + 'static,
    ) -> Self {
        self.on_feedback = Some(Box::new(f));
        self
    }

    /// Finishes the surface. Missing closures become no-ops (events:
    /// always `None`; feedback: accepted silently).
    pub fn build(self) -> CustomSurface {
        CustomSurface {
            name: self.name,
            on_init: self.on_init,
            on_read: self.on_read,
            on_feedback: self.on_feedback,
        }
    }
}

impl ControlSurface for CustomSurface {
    fn name(&self) -> &str {
        &self.name
    }

    fn init(&mut self) -> Result<(), ControlError> {
        match self.on_init.as_mut() {
            Some(f) => f(),
            None => Ok(()),
        }
    }

    fn read_event(&mut self) -> Result<Option<ControlEvent>, ControlError> {
        match self.on_read.as_mut() {
            Some(f) => f(),
            None => Ok(None),
        }
    }

    fn send_feedback(&mut self, feedback: &Feedback) -> Result<(), ControlError> {
        match self.on_feedback.as_mut() {
            Some(f) => f(feedback),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn closures_drive_the_surface() {
        let inits = Arc::new(AtomicUsize::new(0));
        let inits2 = Arc::clone(&inits);
        let mut events = vec![
            Some(ControlEvent::ButtonPress { button: 3 }),
            Some(ControlEvent::ButtonRelease { button: 3 }),
            None,
        ]
        .into_iter();

        let mut surface = CustomSurface::builder("panel")
            .on_init(move || {
                inits2.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .on_read(move || Ok(events.next().unwrap_or(None)))
            .on_feedback(|fb| match fb {
                Feedback::Led { led, on } => {
                    assert_eq!((led, on), (&7, &true));
                    Ok(())
                }
                _ => Ok(()),
            })
            .build();

        assert_eq!(surface.name(), "panel");
        surface.init().unwrap();
        assert_eq!(inits.load(Ordering::SeqCst), 1);
        assert_eq!(
            surface.read_event().unwrap(),
            Some(ControlEvent::ButtonPress { button: 3 })
        );
        assert_eq!(
            surface.read_event().unwrap(),
            Some(ControlEvent::ButtonRelease { button: 3 })
        );
        assert_eq!(surface.read_event().unwrap(), None);
        surface
            .send_feedback(&Feedback::Led { led: 7, on: true })
            .unwrap();
    }

    #[test]
    fn missing_closures_are_noops() {
        let mut surface = CustomSurface::builder("minimal").build();
        surface.init().unwrap();
        assert_eq!(surface.read_event().unwrap(), None);
        surface
            .send_feedback(&Feedback::DisplayText {
                display: 0,
                text: "hi".into(),
            })
            .unwrap();
    }
}
