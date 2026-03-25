//! Trade route data model and fleet management.

use serde::{Deserialize, Serialize};

use super::resources::{ResourceInventory, ResourceType};

/// Unique identifier for a trade ship.
pub type TradeShipId = u64;

/// Unique identifier for a trade route.
pub type TradeRouteId = u64;

/// Current state of a trade ship.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradeShipState {
    /// Ship is stationed at a colony (or Earth).
    Stationed,
    /// Ship is in transit between bodies.
    InTransit,
}

/// A single leg of a trade route (one hop).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteLeg {
    /// Source body index. None = Earth.
    pub from_body: Option<usize>,
    /// Destination body index. None = Earth.
    pub to_body: Option<usize>,
    /// Delta-v required for this leg (m/s).
    pub delta_v: f64,
    /// Flight time for this leg (seconds).
    pub flight_time: f64,
}

/// Cargo manifest: what resources to carry.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CargoManifest {
    pub items: Vec<(ResourceType, f64)>,
}

impl CargoManifest {
    pub fn total_mass(&self) -> f64 {
        self.items.iter().map(|(_, kg)| kg).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty() || self.items.iter().all(|(_, kg)| *kg <= 0.0)
    }
}

/// How a route is automated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AutomationMode {
    /// Manual launch only.
    Manual,
    /// Launch at the next transfer window.
    WindowBased,
    /// Launch every N days.
    FrequencyBased,
    /// Launch when current delta-v cost drops below threshold.
    DvThreshold,
}

impl Default for AutomationMode {
    fn default() -> Self {
        Self::Manual
    }
}

/// A trade route connecting two (or more) bodies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeRoute {
    pub id: TradeRouteId,
    pub name: String,
    /// Route legs (ordered hops).
    pub legs: Vec<RouteLeg>,
    /// Blueprint used for ships on this route.
    pub blueprint_name: String,
    /// Cargo to carry on outbound trip.
    pub outbound_cargo: CargoManifest,
    /// Cargo to carry on return trip.
    pub return_cargo: CargoManifest,
    /// Crew to send with the ship.
    pub crew: u32,
    /// Automation settings.
    pub automation: AutomationMode,
    /// For FrequencyBased: days between launches.
    pub frequency_days: f64,
    /// For DvThreshold: max delta-v to accept (m/s).
    pub dv_threshold: f64,
    /// Priority (higher = processed first).
    pub priority: i32,
    /// Minimum stockpile to keep at source (don't ship below this).
    pub min_stockpile: f64,
    /// Whether route is paused.
    pub paused: bool,
    /// Sim time of last launch (seconds).
    pub last_launch_time: f64,
    /// Ship assigned to this route (if any).
    pub assigned_ship_id: Option<TradeShipId>,
    /// Total route delta-v (sum of all legs, for display).
    pub total_delta_v: f64,
    /// Total route flight time (sum of all legs, for display).
    pub total_flight_time: f64,
    /// Max cargo capacity computed for this route (kg).
    pub max_cargo_capacity: f64,
    /// Route category (SameSOI / Interplanetary / Interstellar).
    #[serde(default)]
    pub route_category: super::transfer::RouteCategory,
    /// For SameSOI/Interstellar: launch interval in days.
    #[serde(default = "default_interval_days")]
    pub interval_days: f64,
    /// For Interplanetary: ships to launch per transfer window.
    #[serde(default = "default_ships_per_window")]
    pub ships_per_window: u32,
    /// Alert reason when automation fails (e.g. insufficient resources).
    /// Route stays active but shows this alert until a launch succeeds.
    #[serde(default)]
    pub alert_reason: Option<String>,
}

fn default_interval_days() -> f64 {
    30.0
}
fn default_ships_per_window() -> u32 {
    1
}

/// A trade ship — an abstract vessel that flies computed routes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeShip {
    pub id: TradeShipId,
    pub name: String,
    /// Blueprint this ship was built from.
    pub blueprint_name: String,
    /// Current state.
    pub state: TradeShipState,
    /// Current location (body index). None = Earth.
    pub location: Option<usize>,
    /// Where the ship started from before current transit.
    pub origin: Option<usize>,
    /// Current cargo being carried.
    pub cargo: CargoManifest,
    /// Crew aboard.
    pub crew: u32,
    /// Which leg of the route the ship is on (index into route legs).
    pub current_leg: usize,
    /// Whether we're on the return trip.
    pub returning: bool,
    /// Remaining transit time (seconds).
    pub transit_remaining: f64,
    /// Route this ship is assigned to.
    pub assigned_route: Option<TradeRouteId>,
    /// Cached total delta-v of this ship's blueprint (m/s).
    pub cached_delta_v: f64,
}

/// Manages all trade ships and routes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FleetManager {
    pub ships: Vec<TradeShip>,
    pub routes: Vec<TradeRoute>,
    pub next_ship_id: TradeShipId,
    pub next_route_id: TradeRouteId,
}

impl FleetManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a ship by ID.
    pub fn get_ship(&self, id: TradeShipId) -> Option<&TradeShip> {
        self.ships.iter().find(|s| s.id == id)
    }

    /// Get a mutable ship by ID.
    pub fn get_ship_mut(&mut self, id: TradeShipId) -> Option<&mut TradeShip> {
        self.ships.iter_mut().find(|s| s.id == id)
    }

    /// Get a route by ID.
    pub fn get_route(&self, id: TradeRouteId) -> Option<&TradeRoute> {
        self.routes.iter().find(|r| r.id == id)
    }

    /// Get a mutable route by ID.
    pub fn get_route_mut(&mut self, id: TradeRouteId) -> Option<&mut TradeRoute> {
        self.routes.iter_mut().find(|r| r.id == id)
    }

    /// Create a new trade ship (already built, stationed at location).
    pub fn create_ship(
        &mut self,
        name: String,
        blueprint_name: String,
        location: Option<usize>,
        cached_delta_v: f64,
    ) -> TradeShipId {
        let id = self.next_ship_id;
        self.next_ship_id += 1;
        self.ships.push(TradeShip {
            id,
            name,
            blueprint_name,
            state: TradeShipState::Stationed,
            location,
            origin: None,
            cargo: CargoManifest::default(),
            crew: 0,
            current_leg: 0,
            returning: false,
            transit_remaining: 0.0,
            assigned_route: None,
            cached_delta_v,
        });
        id
    }

    /// Create a new trade route and return its ID.
    pub fn create_route(&mut self, mut route: TradeRoute) -> TradeRouteId {
        let id = self.next_route_id;
        self.next_route_id += 1;
        route.id = id;
        self.routes.push(route);
        id
    }

    /// Delete a route by ID.
    pub fn delete_route(&mut self, id: TradeRouteId) {
        // Unassign any ships from this route
        for ship in &mut self.ships {
            if ship.assigned_route == Some(id) {
                ship.assigned_route = None;
            }
        }
        self.routes.retain(|r| r.id != id);
    }

    /// Delete a ship by ID.
    pub fn delete_ship(&mut self, id: TradeShipId) {
        // Unassign from any route
        for route in &mut self.routes {
            if route.assigned_ship_id == Some(id) {
                route.assigned_ship_id = None;
            }
        }
        self.ships.retain(|s| s.id != id);
    }

    /// Ships stationed at a given location (body index or None=Earth).
    pub fn ships_at(&self, location: Option<usize>) -> Vec<&TradeShip> {
        self.ships
            .iter()
            .filter(|s| s.state == TradeShipState::Stationed && s.location == location)
            .collect()
    }

    /// Routes that originate from or terminate at a given body.
    pub fn routes_involving(&self, body: Option<usize>) -> Vec<&TradeRoute> {
        self.routes
            .iter()
            .filter(|r| {
                r.legs.iter().any(|leg| leg.from_body == body || leg.to_body == body)
            })
            .collect()
    }

    /// Get the source body of a route (from_body of the first leg).
    pub fn route_source(route: &TradeRoute) -> Option<usize> {
        route.legs.first().and_then(|l| l.from_body)
    }

    /// Get the destination body of a route (to_body of the last leg).
    pub fn route_destination(route: &TradeRoute) -> Option<usize> {
        route.legs.last().and_then(|l| l.to_body)
    }

    /// Propellant resources needed by a blueprint for a full fuel load.
    /// Returns vec of (ResourceType, kg) for each fuel type in the vessel.
    pub fn fuel_requirements_for_blueprint(
        blueprint: &crate::parts::VesselBlueprint,
        part_defs: &crate::parts::PartDefinitions,
    ) -> Vec<(ResourceType, f64)> {
        use crate::parts::FuelType;

        let mut fuel_totals: std::collections::HashMap<ResourceType, f64> =
            std::collections::HashMap::new();

        for placed in &blueprint.parts {
            let Some(def) = part_defs.get(&placed.definition_id) else {
                continue;
            };
            let Some(ref tank) = def.tank else { continue };

            // Use TankData::propellant_capacity() which handles the grid_area calculation
            let (ox_kg, fuel_kg) = tank.propellant_capacity(placed.fuel_type);

            // Map FuelType to ResourceType
            let fuel_rt = match placed.fuel_type {
                FuelType::Rp1 => Some(ResourceType::Rp1),
                FuelType::Methane => Some(ResourceType::Methane),
                FuelType::Hydrogen | FuelType::PureHydrogen => Some(ResourceType::LiquidHydrogen),
                FuelType::Xenon => Some(ResourceType::Xenon),
                FuelType::Monopropellant => None, // Not tracked as colony resource
                FuelType::FusionFuel => Some(ResourceType::Deuterium), // Simplification
                FuelType::Antimatter => Some(ResourceType::Antimatter),
                FuelType::NuclearPulse => Some(ResourceType::NuclearPulseUnits),
                FuelType::Empty => None,
            };

            if let Some(rt) = fuel_rt {
                *fuel_totals.entry(rt).or_default() += fuel_kg;
            }
            if ox_kg > 0.0 {
                *fuel_totals.entry(ResourceType::Lox).or_default() += ox_kg;
            }
        }

        fuel_totals.into_iter().collect()
    }

    /// Load cargo from a colony's inventory into a manifest, respecting min_stockpile.
    /// Returns the actual cargo loaded.
    pub fn load_cargo_from_colony(
        manifest: &CargoManifest,
        colony_resources: &mut ResourceInventory,
        min_stockpile: f64,
    ) -> CargoManifest {
        let mut loaded = CargoManifest::default();
        for &(resource, requested) in &manifest.items {
            let available = colony_resources.get(resource);
            let can_take = (available - min_stockpile).max(0.0).min(requested);
            if can_take > 0.0 {
                colony_resources.remove(resource, can_take);
                loaded.items.push((resource, can_take));
            }
        }
        loaded
    }

    /// Unload cargo into a colony's inventory.
    pub fn unload_cargo_to_colony(
        cargo: &CargoManifest,
        colony_resources: &mut ResourceInventory,
    ) {
        for &(resource, amount) in &cargo.items {
            if amount > 0.0 {
                colony_resources.add(resource, amount);
            }
        }
    }

    /// Sell cargo on Earth for money. Returns total revenue.
    pub fn sell_cargo_on_earth(cargo: &CargoManifest) -> f64 {
        let mut revenue = 0.0;
        for &(resource, amount) in &cargo.items {
            if let Some(price) = resource.earth_price() {
                revenue += price * amount;
            }
        }
        revenue
    }

    // ===== Fleet Transit Simulation (Task 4) =====

    /// Update all in-transit ships. Called once per colony tick from Game::update_colonies().
    pub fn update_fleet(
        &mut self,
        dt_days: f64,
        sim_time: f64,
        colony_manager: &mut super::ColonyManager,
        company: &mut super::Company,
        earth_index: usize,
        body_names: &[String],
        notifications: &mut Vec<super::Notification>,
    ) {
        let dt_seconds = dt_days * 86400.0;

        // Collect ship IDs that need arrival processing
        let mut arrived_ship_ids = Vec::new();

        for ship in &mut self.ships {
            if ship.state == TradeShipState::InTransit {
                ship.transit_remaining -= dt_seconds;
                if ship.transit_remaining <= 0.0 {
                    ship.transit_remaining = 0.0;
                    arrived_ship_ids.push(ship.id);
                }
            }
        }

        // Process arrivals
        for ship_id in arrived_ship_ids {
            self.process_arrival(
                ship_id,
                colony_manager,
                company,
                earth_index,
                body_names,
                notifications,
                sim_time,
            );
        }
    }

    /// Process a ship arriving at its destination.
    /// Routes are one-way: ship delivers cargo and stays at destination.
    fn process_arrival(
        &mut self,
        ship_id: TradeShipId,
        colony_manager: &mut super::ColonyManager,
        company: &mut super::Company,
        earth_index: usize,
        body_names: &[String],
        notifications: &mut Vec<super::Notification>,
        sim_time: f64,
    ) {
        // Find the ship and its route
        let ship_idx = match self.ships.iter().position(|s| s.id == ship_id) {
            Some(idx) => idx,
            None => return,
        };

        let route_id = self.ships[ship_idx].assigned_route;
        let route = route_id.and_then(|id| self.routes.iter().find(|r| r.id == id));

        // Determine current leg's destination
        let destination = if let Some(route) = route {
            let leg_idx = self.ships[ship_idx].current_leg;
            route.legs.get(leg_idx).and_then(|leg| leg.to_body)
        } else {
            self.ships[ship_idx].origin
        };

        let dest_name = match destination {
            Some(idx) if idx < body_names.len() => body_names[idx].clone(),
            None => "Earth".to_string(),
            _ => "Unknown".to_string(),
        };

        // Check if destination is Earth
        let is_earth = destination.map_or(true, |idx| idx == earth_index);

        // Unload cargo
        let cargo = std::mem::take(&mut self.ships[ship_idx].cargo);
        if is_earth {
            // Sell on Earth
            let revenue = Self::sell_cargo_on_earth(&cargo);
            company.money += revenue;
        } else if let Some(dest_idx) = destination {
            // Unload to colony
            if let Some(colony) = colony_manager.get_by_body_mut(dest_idx) {
                Self::unload_cargo_to_colony(&cargo, &mut colony.resources);
                // Return crew to colony
                let crew = self.ships[ship_idx].crew;
                if crew > 0 {
                    colony.crew += crew;
                    self.ships[ship_idx].crew = 0;
                }
            }
        }

        // Update ship location
        self.ships[ship_idx].location = destination;
        self.ships[ship_idx].origin = None;

        // Check if there are more legs (multi-hop)
        let route_info = route_id.and_then(|id| self.routes.iter().find(|r| r.id == id)).cloned();
        if let Some(route) = &route_info {
            let next_leg = self.ships[ship_idx].current_leg + 1;
            if next_leg < route.legs.len() {
                if let Some(leg) = route.legs.get(next_leg).cloned() {
                    self.ships[ship_idx].current_leg = next_leg;
                    self.ships[ship_idx].transit_remaining = leg.flight_time;
                    self.ships[ship_idx].origin = self.ships[ship_idx].location;
                    return;
                }
            }
        }

        // Ship has arrived at final destination — station it here
        self.ships[ship_idx].state = TradeShipState::Stationed;
        self.ships[ship_idx].returning = false;
        self.ships[ship_idx].current_leg = 0;

        // Unassign ship from route (one-way: a new ship will be built for next launch)
        self.ships[ship_idx].assigned_route = None;
        if let Some(rid) = route_id {
            if let Some(route) = self.routes.iter_mut().find(|r| r.id == rid) {
                route.assigned_ship_id = None;
            }
        }

        let ship_name = self.ships[ship_idx].name.clone();
        notifications.push(super::Notification {
            kind: super::notification::NotificationKind::ShipArrived {
                ship_name,
                destination: dest_name,
            },
            time: sim_time,
            read: false,
        });
    }

    /// Launch a ship on a route. Consumes fuel and loads cargo from source.
    pub fn launch_ship(
        &mut self,
        ship_id: TradeShipId,
        route_id: TradeRouteId,
        colony_manager: &mut super::ColonyManager,
        company: &mut super::Company,
        earth_index: usize,
        blueprints: &crate::parts::BlueprintRegistry,
        part_defs: &crate::parts::PartDefinitions,
        _body_names: &[String],
        notifications: &mut Vec<super::Notification>,
        sim_time: f64,
    ) -> Result<(), String> {
        // Validate ship exists and is stationed
        let ship = self.get_ship(ship_id).ok_or("Ship not found")?;
        if ship.state != TradeShipState::Stationed {
            return Err("Ship is not stationed".to_string());
        }

        let route = self.get_route(route_id).ok_or("Route not found")?;
        let first_leg = route.legs.first().ok_or("Route has no legs")?;

        // Validate ship is at the route source
        let route_source = first_leg.from_body;
        if ship.location != route_source {
            return Err("Ship is not at route source".to_string());
        }

        let is_earth_source = route_source.map_or(true, |idx| idx == earth_index);

        // Get fuel requirements
        let blueprint = blueprints
            .get(&ship.blueprint_name)
            .ok_or("Blueprint not found")?;
        let fuel_reqs = Self::fuel_requirements_for_blueprint(blueprint, part_defs);

        // Consume fuel (rocket cost is already paid by build_ship)
        if is_earth_source {
            // Buy fuel from Earth
            let mut total_fuel_cost = 0.0;
            for &(resource, amount) in &fuel_reqs {
                if let Some(price) = resource.earth_price() {
                    total_fuel_cost += price * amount;
                } else {
                    return Err(format!("Cannot buy {} on Earth", resource.display_name()));
                }
            }

            if company.money < total_fuel_cost {
                return Err(format!("Insufficient funds for fuel (need {}, have {})",
                    super::format_money(total_fuel_cost),
                    super::format_money(company.money)));
            }
            company.money -= total_fuel_cost;
        } else if let Some(source_idx) = route_source {
            // Consume fuel from colony
            let colony = colony_manager
                .get_by_body_mut(source_idx)
                .ok_or("No colony at route source")?;
            for &(resource, amount) in &fuel_reqs {
                if !colony.resources.has_enough(resource, amount) {
                    return Err(format!(
                        "Insufficient {} at colony (need {:.0} kg)",
                        resource.display_name(),
                        amount
                    ));
                }
            }
            for &(resource, amount) in &fuel_reqs {
                colony.resources.remove(resource, amount);
            }
        }

        // Load cargo
        let route = self.get_route(route_id).ok_or("Route not found")?;
        let outbound_cargo = route.outbound_cargo.clone();
        let min_stockpile = route.min_stockpile;
        let crew_to_send = route.crew;
        let flight_time = first_leg.flight_time;
        let route_name = route.name.clone();

        let loaded_cargo = if is_earth_source {
            // Buy cargo from Earth — pre-check total cost
            let mut cargo_cost = 0.0;
            for &(resource, amount) in &outbound_cargo.items {
                if let Some(price) = resource.earth_price() {
                    cargo_cost += price * amount;
                }
            }
            if cargo_cost > 0.0 && company.money < cargo_cost {
                return Err(format!("Insufficient funds for cargo (need {}, have {})",
                    super::format_money(cargo_cost),
                    super::format_money(company.money)));
            }
            company.money -= cargo_cost;
            CargoManifest { items: outbound_cargo.items.clone() }
        } else if let Some(source_idx) = route_source {
            let colony = colony_manager
                .get_by_body_mut(source_idx)
                .ok_or("No colony at route source")?;

            // Assign crew from colony
            if crew_to_send > 0 {
                if colony.crew < crew_to_send {
                    return Err(format!(
                        "Insufficient crew at colony (need {}, have {})",
                        crew_to_send, colony.crew
                    ));
                }
                colony.crew -= crew_to_send;
            }

            Self::load_cargo_from_colony(&outbound_cargo, &mut colony.resources, min_stockpile)
        } else {
            CargoManifest::default()
        };

        // Subtract food for journey (consumed in transit, not added to cargo)
        let flight_days = flight_time / 86400.0;
        let food_kg = crew_to_send as f64 * 0.5 * flight_days;
        if food_kg > 0.0 {
            if is_earth_source {
                // Buy food from Earth
                let food_price = ResourceType::Food.earth_price().unwrap_or(50.0);
                let food_cost = food_kg * food_price;
                if company.money < food_cost {
                    return Err(format!(
                        "Insufficient funds for food (need {})",
                        super::format_money(food_cost)
                    ));
                }
                company.money -= food_cost;
            } else if let Some(source_idx) = route_source {
                let colony = colony_manager
                    .get_by_body_mut(source_idx)
                    .ok_or("No colony at route source")?;
                if !colony.resources.has_enough(ResourceType::Food, food_kg) {
                    return Err(format!(
                        "Insufficient Food at colony (need {:.0} kg, have {:.0} kg)",
                        food_kg,
                        colony.resources.get(ResourceType::Food)
                    ));
                }
                colony.resources.remove(ResourceType::Food, food_kg);
            }
        }

        // Update ship state
        let ship = self.get_ship_mut(ship_id).ok_or("Ship not found")?;
        ship.state = TradeShipState::InTransit;
        ship.cargo = loaded_cargo;
        ship.crew = crew_to_send;
        ship.current_leg = 0;
        ship.returning = false;
        ship.transit_remaining = flight_time;
        ship.origin = ship.location;
        ship.assigned_route = Some(route_id);

        // Update route
        if let Some(route) = self.get_route_mut(route_id) {
            route.assigned_ship_id = Some(ship_id);
            route.last_launch_time = sim_time;
        }

        let ship_name = self.get_ship(ship_id).map(|s| s.name.clone()).unwrap_or_default();
        notifications.push(super::Notification {
            kind: super::notification::NotificationKind::ShipDeparted {
                ship_name,
                route_name,
            },
            time: sim_time,
            read: false,
        });

        Ok(())
    }

    /// Build a new trade ship at a location. Deducts money (Earth) or resources (colony).
    pub fn build_ship(
        &mut self,
        name: String,
        blueprint_name: String,
        location: Option<usize>,
        earth_index: usize,
        blueprints: &crate::parts::BlueprintRegistry,
        part_defs: &crate::parts::PartDefinitions,
        colony_manager: &mut super::ColonyManager,
        company: &mut super::Company,
        body_names: &[String],
        notifications: &mut Vec<super::Notification>,
        sim_time: f64,
    ) -> Result<TradeShipId, String> {
        let blueprint = blueprints
            .get(&blueprint_name)
            .ok_or("Blueprint not found")?;

        let is_earth = location.map_or(true, |idx| idx == earth_index);

        if is_earth {
            // Compute cost in dollars (sum of part costs)
            let cost: f64 = blueprint.parts.iter()
                .filter_map(|p| part_defs.get(&p.definition_id))
                .map(|d| d.cost as f64)
                .sum();
            if company.money < cost {
                return Err(format!("Insufficient funds (need {})",
                    super::format_money(cost)));
            }
            company.money -= cost;
        } else {
            // At colony: require structural metal proportional to vessel mass
            let total_mass_kg: f64 = blueprint.parts.iter()
                .filter_map(|p| part_defs.get(&p.definition_id))
                .map(|d| d.mass * 1000.0) // tonnes to kg
                .sum();
            let metal_needed = total_mass_kg * 2.0; // 2x mass in structural metal

            if let Some(body_idx) = location {
                let colony = colony_manager
                    .get_by_body_mut(body_idx)
                    .ok_or("No colony at location")?;
                if !colony.resources.has_enough(ResourceType::StructuralMetal, metal_needed) {
                    return Err(format!(
                        "Insufficient structural metal (need {:.0} kg)",
                        metal_needed
                    ));
                }
                colony.resources.remove(ResourceType::StructuralMetal, metal_needed);
            }
        }

        // Compute cached delta-v
        let cached_dv = super::transfer::blueprint_total_delta_v(blueprint, part_defs);

        let ship_id = self.create_ship(name.clone(), blueprint_name, location, cached_dv);

        let location_name = match location {
            Some(idx) if idx < body_names.len() => body_names[idx].clone(),
            None => "Earth".to_string(),
            _ => "Unknown".to_string(),
        };

        notifications.push(super::Notification {
            kind: super::notification::NotificationKind::ShipConstructionComplete {
                ship_name: name,
                location: location_name,
            },
            time: sim_time,
            read: false,
        });

        Ok(ship_id)
    }

    // ===== Route Automation (Task 5) =====

    /// Check automation rules and launch ships when conditions are met.
    /// Called from update_fleet() once per tick.
    ///
    /// Routes are one-way: each launch builds a new ship, flies it to the destination,
    /// and the ship stays there. When the interval triggers again, a new ship is built.
    pub fn check_automation(
        &mut self,
        sim_time: f64,
        colony_manager: &mut super::ColonyManager,
        company: &mut super::Company,
        earth_index: usize,
        solar_system: &crate::bodies::SolarSystem,
        body_names: &[String],
        notifications: &mut Vec<super::Notification>,
        blueprints: &crate::parts::BlueprintRegistry,
        part_defs: &crate::parts::PartDefinitions,
    ) {
        // Collect non-paused routes, sorted by priority desc then id asc
        let mut route_ids: Vec<(TradeRouteId, i32)> = self
            .routes
            .iter()
            .filter(|r| !r.paused)
            .map(|r| (r.id, r.priority))
            .collect();
        route_ids.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        for (route_id, _) in route_ids {
            let route = match self.routes.iter().find(|r| r.id == route_id) {
                Some(r) => r,
                None => continue,
            };

            // If a ship is currently in transit on this route, skip
            if let Some(ship_id) = route.assigned_ship_id {
                if let Some(ship) = self.ships.iter().find(|s| s.id == ship_id) {
                    if ship.state == TradeShipState::InTransit {
                        continue;
                    }
                }
            }

            // If route already has an alert (e.g. insufficient funds), don't retry every tick.
            // Alert clears on manual launch or route edit.
            if route.alert_reason.is_some() {
                continue;
            }

            let route_source = route.legs.first().and_then(|l| l.from_body);

            // Check if it's time to launch
            let should_launch = if route.automation == AutomationMode::Manual {
                match route.route_category {
                    super::transfer::RouteCategory::SameSOI | super::transfer::RouteCategory::Interstellar => {
                        let interval_seconds = route.interval_days * 86400.0;
                        if interval_seconds <= 0.0 {
                            false
                        } else {
                            sim_time - route.last_launch_time >= interval_seconds
                        }
                    }
                    super::transfer::RouteCategory::Interplanetary => {
                        let dest = route.legs.last().and_then(|l| l.to_body);
                        if let Some((window_time, _dv)) = super::transfer::next_launch_window(
                            route_source,
                            dest,
                            sim_time,
                            solar_system,
                        ) {
                            (window_time - sim_time).abs() < 86400.0
                        } else {
                            false
                        }
                    }
                }
            } else {
                match route.automation {
                    AutomationMode::Manual => false,
                    AutomationMode::WindowBased => {
                        let dest = route.legs.last().and_then(|l| l.to_body);
                        if let Some((window_time, _dv)) = super::transfer::next_launch_window(
                            route_source,
                            dest,
                            sim_time,
                            solar_system,
                        ) {
                            (window_time - sim_time).abs() < 86400.0
                        } else {
                            false
                        }
                    }
                    AutomationMode::FrequencyBased => {
                        let frequency_seconds = route.frequency_days * 86400.0;
                        if frequency_seconds <= 0.0 {
                            false
                        } else {
                            sim_time - route.last_launch_time >= frequency_seconds
                        }
                    }
                    AutomationMode::DvThreshold => {
                        let dest = route.legs.last().and_then(|l| l.to_body);
                        if let Some(leg) = super::transfer::compute_leg_delta_v(
                            route_source,
                            dest,
                            sim_time,
                            0.0,
                            solar_system,
                        ) {
                            leg.total_dv <= route.dv_threshold
                        } else {
                            false
                        }
                    }
                }
            };

            if !should_launch {
                continue;
            }

            let route_name_for_log = route.name.clone();
            let blueprint_name = route.blueprint_name.clone();
            let had_alert = route.alert_reason.is_some();

            // Build a new ship for this launch (one-way: each launch gets a fresh ship)
            let ship_name = format!("{} #{}", route_name_for_log, sim_time as u64 / 86400);
            log::info!("[trade] Building ship for route '{}'", route_name_for_log);

            let build_result = self.build_ship(
                ship_name,
                blueprint_name,
                route_source,
                earth_index,
                blueprints,
                part_defs,
                colony_manager,
                company,
                body_names,
                notifications,
                sim_time,
            );

            let ship_id = match build_result {
                Ok(id) => {
                    // Assign ship to route
                    if let Some(r) = self.get_route_mut(route_id) {
                        r.assigned_ship_id = Some(id);
                    }
                    if let Some(s) = self.get_ship_mut(id) {
                        s.assigned_route = Some(route_id);
                    }
                    id
                }
                Err(reason) => {
                    log::warn!("[trade] Route '{}': ship build failed: {}", route_name_for_log, reason);
                    if let Some(route) = self.get_route_mut(route_id) {
                        route.alert_reason = Some(reason.clone());
                    }
                    if !had_alert {
                        notifications.push(super::Notification {
                            kind: super::notification::NotificationKind::RoutePaused {
                                route_name: route_name_for_log,
                                reason,
                            },
                            time: sim_time,
                            read: false,
                        });
                    }
                    continue;
                }
            };

            // Launch the ship
            log::info!("[trade] Launching on route '{}'", route_name_for_log);
            let result = self.launch_ship(
                ship_id,
                route_id,
                colony_manager,
                company,
                earth_index,
                blueprints,
                part_defs,
                body_names,
                notifications,
                sim_time,
            );

            match result {
                Err(reason) => {
                    log::warn!("[trade] Route '{}': launch failed: {}", route_name_for_log, reason);
                    if let Some(route) = self.get_route_mut(route_id) {
                        route.alert_reason = Some(reason.clone());
                    }
                    if !had_alert {
                        notifications.push(super::Notification {
                            kind: super::notification::NotificationKind::RoutePaused {
                                route_name: route_name_for_log,
                                reason,
                            },
                            time: sim_time,
                            read: false,
                        });
                    }
                }
                Ok(()) => {
                    log::info!("[trade] Route '{}': launch successful", route_name_for_log);
                    if let Some(route) = self.get_route_mut(route_id) {
                        route.alert_reason = None;
                    }
                }
            }
        }
    }
}
