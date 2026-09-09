//! Where a human meets the system, declared so an agent can reason about it.
//!
//! An agent should drive the API, not the interface. This module exists for the
//! other direction: so an agent can *explain* a screen to the person looking at
//! it, reconcile what the interface offers against what the API permits, and
//! tell the difference between a page that is built and a page that is planned.

use serde::{Deserialize, Serialize};

/// What kind of thing a surface is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurfaceKind {
    /// A page served to a browser.
    ///
    /// The default, because an HTTP surface is what a product had when this
    /// vocabulary described only one.
    #[default]
    Web,
    /// A command-line entry point.
    Cli,
    /// A desktop or native shell.
    Desktop,
    /// A machine interface with no human view — an MCP server, a webhook.
    Machine,
}

impl SurfaceKind {
    /// The name used in findings and in the serialised form.
    pub fn label(self) -> &'static str {
        match self {
            SurfaceKind::Web => "web",
            SurfaceKind::Cli => "cli",
            SurfaceKind::Desktop => "desktop",
            SurfaceKind::Machine => "machine",
        }
    }
}

/// The kind of control an affordance is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Control {
    Text,
    Number,
    Select,
    Toggle,
    /// Triggers the view's capability.
    Submit,
    /// Displays a value; not an input.
    Readout,
    /// Navigates elsewhere.
    Link,
    /// Uploads a file.
    File,
}

/// A condition under which an affordance appears.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    /// The affordance whose value governs this one.
    pub affordance: &'static str,
    pub equals: &'static str,
}

/// One control on a view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Affordance {
    pub id: &'static str,
    pub control: Control,
    pub label: &'static str,
    /// The capability input or returned property this control is bound to.
    pub binds: &'static str,
    pub required: bool,
    /// Shown only when this holds.
    pub shown_when: Option<Condition>,
}

/// One coherent screen or subcommand within a surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub id: &'static str,
    pub purpose: &'static str,
    /// The capability this view exercises. Empty for a purely informational
    /// view that calls nothing.
    pub capability: &'static str,
    pub affordances: Vec<Affordance>,
}

/// A place a human meets the system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Surface {
    pub id: &'static str,
    pub kind: SurfaceKind,
    pub purpose: &'static str,
    /// Whether this exists today.
    ///
    /// Declaring a planned surface as `false` is the point: an agent that finds
    /// it in the model and not in the product has read a plan, not a bug.
    pub implemented: bool,
    /// Why it is not built, when `implemented` is false.
    pub not_built: Option<&'static str>,
    /// Where it lives — a path, a route, a command.
    pub source: Option<&'static str>,
    pub views: Vec<View>,
}

impl Surface {
    pub fn view(&self, id: &str) -> Option<&View> {
        self.views.iter().find(|v| v.id == id)
    }
}
