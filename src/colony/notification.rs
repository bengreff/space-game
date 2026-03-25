use serde::{Deserialize, Serialize};

/// Categories of colony notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationKind {
    ColonyEstablished { colony_name: String },
    ColonyFoodDepleted { colony_name: String },
    ColonyResourceDepleted { colony_name: String, resource: String },
    ColonyPowerLoss { colony_name: String },
    ConstructionComplete { colony_name: String, building: String },
    ShipArrived { ship_name: String, destination: String },
    ShipDeparted { ship_name: String, route_name: String },
    RoutePaused { route_name: String, reason: String },
    ShipConstructionComplete { ship_name: String, location: String },
    ContractCompleted { name: String, payout: f64 },
    MilestoneAchieved { name: String, payout: f64 },
}

impl NotificationKind {
    /// Whether this notification should stop time warp.
    pub fn stops_warp(&self) -> bool {
        matches!(
            self,
            NotificationKind::ColonyFoodDepleted { .. }
                | NotificationKind::ColonyResourceDepleted { .. }
                | NotificationKind::ColonyPowerLoss { .. }
                | NotificationKind::RoutePaused { .. }
        )
    }

    /// Human-readable message for toast display.
    pub fn message(&self) -> String {
        match self {
            NotificationKind::ColonyEstablished { colony_name } => {
                format!("Colony established: {}", colony_name)
            }
            NotificationKind::ColonyFoodDepleted { colony_name } => {
                format!("{}: Food supply exhausted!", colony_name)
            }
            NotificationKind::ColonyResourceDepleted { colony_name, resource } => {
                format!("{}: {} depleted!", colony_name, resource)
            }
            NotificationKind::ColonyPowerLoss { colony_name } => {
                format!("{}: Power generation insufficient!", colony_name)
            }
            NotificationKind::ConstructionComplete { colony_name, building } => {
                format!("{}: {} construction complete", colony_name, building)
            }
            NotificationKind::ShipArrived { ship_name, destination } => {
                format!("{} arrived at {}", ship_name, destination)
            }
            NotificationKind::ShipDeparted { ship_name, route_name } => {
                format!("{} departed on route {}", ship_name, route_name)
            }
            NotificationKind::RoutePaused { route_name, reason } => {
                format!("Route {} paused: {}", route_name, reason)
            }
            NotificationKind::ShipConstructionComplete { ship_name, location } => {
                format!("{} construction complete at {}", ship_name, location)
            }
            NotificationKind::ContractCompleted { name, payout } => {
                format!("Contract complete: {} — {}", name, crate::colony::format_money(*payout))
            }
            NotificationKind::MilestoneAchieved { name, payout } => {
                format!("{} — {}", name, crate::colony::format_money(*payout))
            }
        }
    }
}

/// A game notification (persisted in saves).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Notification {
    pub kind: NotificationKind,
    /// Simulation time when the notification was created.
    pub time: f64,
    /// Whether this notification has been acknowledged/processed.
    pub read: bool,
}

impl Default for Notification {
    fn default() -> Self {
        Self {
            kind: NotificationKind::ContractCompleted { name: String::new(), payout: 0.0 },
            time: 0.0,
            read: true,
        }
    }
}
