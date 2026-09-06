//! Core-owned route matching and memory history projected as plain JSON snapshots.
//!
//! Routers are CPU-only and never reach the application runtime, so the application thread owns
//! them through synchronous calls; each snapshot is one JSON document.

use serde::{Deserialize, Serialize};

use quickgui::{
    RouteDefinition, RouteLocation, RouteMatch, RouteParameter, RouteQueryPair, Router,
    RouterSnapshot,
};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRouteDefinition {
    pub id: String,
    pub path: Option<String>,
    pub parent_id: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct NativeRouteValue {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Serialize)]
pub struct NativeRouteLocation {
    pub href: String,
    pub pathname: String,
    pub search: String,
    pub hash: String,
    pub query: Vec<NativeRouteValue>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRouteMatch {
    pub route_ids: Vec<String>,
    pub params: Vec<NativeRouteValue>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRouterState {
    pub location: NativeRouteLocation,
    pub matched: Option<NativeRouteMatch>,
    pub history_index: u32,
    pub history_length: u32,
    pub can_go_back: bool,
    pub can_go_forward: bool,
}

/// One core router owned by the application thread.
pub struct NativeRouter {
    inner: Router,
}

impl NativeRouter {
    pub fn new(
        routes: Vec<NativeRouteDefinition>,
        initial_destination: Option<String>,
    ) -> Result<Self, String> {
        let routes = routes.into_iter().map(|route| {
            let definition = match route.path {
                Some(path) => RouteDefinition::new(route.id, path),
                None => RouteDefinition::layout(route.id),
            };
            match route.parent_id {
                Some(parent) => definition.parent(parent),
                None => definition,
            }
        });
        Ok(Self {
            inner: Router::new(routes, initial_destination.as_deref().unwrap_or("/"))
                .map_err(|error| error.to_string())?,
        })
    }

    pub fn state(&self) -> NativeRouterState {
        native_router_state(self.inner.snapshot())
    }

    pub fn resolve(&self, destination: &str) -> Result<NativeRouteLocation, String> {
        self.inner
            .resolve(destination)
            .map(native_route_location)
            .map_err(|error| error.to_string())
    }

    pub fn is_active(&self, destination: &str, end: bool) -> Result<bool, String> {
        self.inner
            .is_active(destination, end)
            .map_err(|error| error.to_string())
    }

    pub fn push(&mut self, destination: &str) -> Result<NativeRouterState, String> {
        self.inner
            .push(destination)
            .map_err(|error| error.to_string())?;
        Ok(self.state())
    }

    pub fn replace(&mut self, destination: &str) -> Result<NativeRouterState, String> {
        self.inner
            .replace(destination)
            .map_err(|error| error.to_string())?;
        Ok(self.state())
    }

    pub fn go(&mut self, delta: i32) -> NativeRouterState {
        self.inner.go(delta as isize);
        self.state()
    }

    pub fn back(&mut self) -> NativeRouterState {
        self.inner.back();
        self.state()
    }

    pub fn forward(&mut self) -> NativeRouterState {
        self.inner.forward();
        self.state()
    }
}

fn native_route_values(values: &[RouteParameter]) -> Vec<NativeRouteValue> {
    values
        .iter()
        .map(|parameter| NativeRouteValue {
            name: parameter.name().to_owned(),
            value: parameter.value().to_owned(),
        })
        .collect()
}

fn native_query_values(values: &[RouteQueryPair]) -> Vec<NativeRouteValue> {
    values
        .iter()
        .map(|pair| NativeRouteValue {
            name: pair.name().to_owned(),
            value: pair.value().to_owned(),
        })
        .collect()
}

fn native_route_location(location: RouteLocation) -> NativeRouteLocation {
    NativeRouteLocation {
        href: location.href().to_owned(),
        pathname: location.pathname().to_owned(),
        search: location.search().to_owned(),
        hash: location.hash().to_owned(),
        query: native_query_values(location.query()),
    }
}

fn native_route_match(matched: RouteMatch) -> NativeRouteMatch {
    NativeRouteMatch {
        route_ids: matched
            .route_ids()
            .iter()
            .map(|id| id.to_string())
            .collect(),
        params: native_route_values(matched.params()),
    }
}

fn native_router_state(snapshot: RouterSnapshot) -> NativeRouterState {
    NativeRouterState {
        location: native_route_location(snapshot.location),
        matched: snapshot.matched.map(native_route_match),
        history_index: snapshot.history_index as u32,
        history_length: snapshot.history_length as u32,
        can_go_back: snapshot.can_go_back,
        can_go_forward: snapshot.can_go_forward,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_projects_core_matches_and_history() {
        let mut router = NativeRouter::new(
            vec![
                NativeRouteDefinition {
                    id: "shell".to_owned(),
                    path: None,
                    parent_id: None,
                },
                NativeRouteDefinition {
                    id: "project".to_owned(),
                    path: Some("/projects/:id".to_owned()),
                    parent_id: Some("shell".to_owned()),
                },
            ],
            Some("/projects/quickgui?tab=activity".to_owned()),
        )
        .unwrap();

        let state = router.state();
        let matched = state.matched.unwrap();
        assert_eq!(matched.route_ids, ["shell", "project"]);
        assert_eq!(matched.params[0].name, "id");
        assert_eq!(matched.params[0].value, "quickgui");
        assert_eq!(state.location.query[0].name, "tab");
        assert_eq!(state.location.query[0].value, "activity");

        let state = router.push("/projects/second").unwrap();
        assert!(state.can_go_back);
        let state = router.back();
        assert_eq!(state.location.href, "/projects/quickgui?tab=activity");
    }
}
