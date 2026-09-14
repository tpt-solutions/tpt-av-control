//! Message routing: match OSC address patterns to handlers.

use crate::address::OscAddressMatcher;
use crate::bundle::OscPacket;
use crate::message::OscMessage;

type OscRouteHandler = Box<dyn FnMut(OscMessage) + Send>;

struct Route {
    matcher: OscAddressMatcher,
    handler: OscRouteHandler,
}

/// Routes incoming OSC messages to handlers by address pattern.
///
/// Routes are evaluated in registration order; a message is delivered to
/// every matching route (OSC semantics allow multiple matches).
pub struct OscDispatcher {
    routes: Vec<Route>,
}

impl Default for OscDispatcher {
    fn default() -> Self {
        Self { routes: Vec::new() }
    }
}

impl OscDispatcher {
    /// Creates an empty dispatcher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handler for `pattern`. Replaces any existing route with
    /// the same pattern.
    pub fn add_route(
        &mut self,
        pattern: &str,
        handler: impl FnMut(OscMessage) + Send + 'static,
    ) {
        self.remove_route(pattern);
        self.routes.push(Route {
            matcher: OscAddressMatcher::new(pattern),
            handler: Box::new(handler),
        });
    }

    /// Removes the route for `pattern`. Returns whether one existed.
    pub fn remove_route(&mut self, pattern: &str) -> bool {
        let before = self.routes.len();
        self.routes
            .retain(|r| r.matcher.pattern() != pattern);
        self.routes.len() != before
    }

    /// The registered patterns, in registration order.
    pub fn patterns(&self) -> impl Iterator<Item = &str> {
        self.routes.iter().map(|r| r.matcher.pattern())
    }

    /// Whether no routes are registered.
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// Dispatches a message to all matching routes. Returns how many
    /// handlers ran.
    pub fn dispatch(&mut self, message: OscMessage) -> usize {
        let mut handled = 0;
        for route in &mut self.routes {
            if route.matcher.matches(&message.address) {
                (route.handler)(message.clone());
                handled += 1;
            }
        }
        handled
    }

    /// Dispatches a packet (expanding bundles recursively). Returns how
    /// many handler invocations happened in total.
    pub fn dispatch_packet(&mut self, packet: &OscPacket) -> usize {
        match packet {
            OscPacket::Message(m) => self.dispatch(m.clone()),
            OscPacket::Bundle(b) => b
                .elements
                .iter()
                .map(|element| self.dispatch_packet(element))
                .sum(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::OscArg;
    use std::sync::{Arc, Mutex};

    fn recorder() -> (Arc<Mutex<Vec<String>>>, impl FnMut(OscMessage) + Send) {
        let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let l2 = Arc::clone(&log);
        (log, move |msg: OscMessage| {
            l2.lock().unwrap().push(msg.address);
        })
    }

    #[test]
    fn routes_match_patterns() {
        let mut d = OscDispatcher::new();
        assert!(d.is_empty());
        let (log_a, h_a) = recorder();
        let (log_b, h_b) = recorder();
        d.add_route("/track/*/volume", h_a);
        d.add_route("/master/*", h_b);
        assert_eq!(d.patterns().count(), 2);

        assert_eq!(d.dispatch(OscMessage::new("/track/3/volume", &[]).unwrap()), 1);
        assert_eq!(d.dispatch(OscMessage::new("/master/gain", &[]).unwrap()), 1);
        assert_eq!(d.dispatch(OscMessage::new("/other", &[]).unwrap()), 0);

        assert_eq!(log_a.lock().unwrap().as_slice(), ["/track/3/volume"]);
        assert_eq!(log_b.lock().unwrap().as_slice(), ["/master/gain"]);
    }

    #[test]
    fn overlapping_routes_all_fire() {
        let mut d = OscDispatcher::new();
        let (log_a, h_a) = recorder();
        let (log_b, h_b) = recorder();
        d.add_route("/x", h_a);
        d.add_route("/*", h_b);
        let hits = d.dispatch(OscMessage::new_unchecked("/x", vec![]));
        assert_eq!(hits, 2);
        assert_eq!(log_a.lock().unwrap().len(), 1);
        assert_eq!(log_b.lock().unwrap().len(), 1);
    }

    #[test]
    fn add_route_replaces_same_pattern() {
        let mut d = OscDispatcher::new();
        let (_log_a, h_a) = recorder();
        let (log_b, h_b) = recorder();
        d.add_route("/x", h_a);
        d.add_route("/x", h_b);
        assert_eq!(d.patterns().count(), 1);
        d.dispatch(OscMessage::new_unchecked("/x", vec![OscArg::Nil]));
        assert_eq!(log_b.lock().unwrap().len(), 1);
    }

    #[test]
    fn remove_route() {
        let mut d = OscDispatcher::new();
        let (_log, h) = recorder();
        d.add_route("/y", h);
        assert!(d.remove_route("/y"));
        assert!(!d.remove_route("/y"));
        assert!(d.is_empty());
    }

    #[test]
    fn dispatches_bundles() {
        let mut d = OscDispatcher::new();
        let (log, h) = recorder();
        d.add_route("/*", h);
        let bundle = OscPacket::Bundle(crate::bundle::OscBundle::new(
            None,
            vec![
                OscPacket::Message(OscMessage::new_unchecked("/a", vec![])),
                OscPacket::Bundle(crate::bundle::OscBundle::new(
                    None,
                    vec![OscPacket::Message(OscMessage::new_unchecked("/b", vec![]))],
                )),
            ],
        ));
        assert_eq!(d.dispatch_packet(&bundle), 2);
        assert_eq!(log.lock().unwrap().as_slice(), ["/a", "/b"]);
    }
}
