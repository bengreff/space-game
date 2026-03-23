use serde::{Deserialize, Serialize};

/// Categories of colony notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationKind {
    ColonyEstablished { colony_name: String },
    ColonyFoodDepleted { colony_name: String },
    ColonyResourceDepleted { colony_name: String, resource: String },
    ColonyPowerLoss { colony_name: String },
    ConstructionComplete { colony_name: String, building: String },
}

impl NotificationKind {
    /// Whether this notification should stop time warp.
    pub fn stops_warp(&self) -> bool {
        matches!(
            self,
            NotificationKind::ColonyFoodDepleted { .. }
                | NotificationKind::ColonyResourceDepleted { .. }
                | NotificationKind::ColonyPowerLoss { .. }
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
        }
    }
}

/// A game notification (persisted in saves).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub kind: NotificationKind,
    /// Simulation time when the notification was created.
    pub time: f64,
    /// Whether this notification has been acknowledged/processed.
    pub read: bool,
}
