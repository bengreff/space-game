use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::colony::{ColonyManager, Company, Notification, ScienceState};
use crate::game::{Game, VesselId, TrackedVessel};
use crate::parts::{FlightVessel, VesselBlueprint};
use crate::render::ManeuverNode;
use crate::ship::Ship;

const SAVE_DIR: &str = "data/saves";
const SAVE_VERSION: u32 = 1;

/// Write a file atomically: write to a temp file in the same directory, then rename.
/// Rename is atomic on all platforms, so a crash mid-write can never corrupt the target.
fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let tmp_path = path.with_extension("ron.tmp");
    fs::write(&tmp_path, content)
        .map_err(|e| format!("Failed to write temp file {:?}: {}", tmp_path, e))?;
    fs::rename(&tmp_path, path)
        .map_err(|e| format!("Failed to rename {:?} -> {:?}: {}", tmp_path, path, e))?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct SaveGame {
    pub version: u32,
    pub name: String,
    pub simulation_time: f64,
    pub vessels: Vec<SavedVessel>,
    pub next_vessel_id: VesselId,
    pub debris_counter: u32,
    pub blueprints: Vec<VesselBlueprint>,
    pub editor_vessel_name: String,
    // Colony system state (all default for backward compat with old saves)
    #[serde(default)]
    pub colonies: ColonyManager,
    #[serde(default)]
    pub company: Company,
    #[serde(default)]
    pub science: ScienceState,
    #[serde(default)]
    pub tech_unlocked: HashSet<String>,
    #[serde(default)]
    pub tech_line_tiers: HashMap<String, u32>,
    #[serde(default)]
    pub notifications: Vec<Notification>,
    /// Editor blueprint at time of save (used by "Revert to Editor")
    #[serde(default)]
    pub editor_blueprint: Option<VesselBlueprint>,
}

#[derive(Serialize, Deserialize)]
pub struct SavedVessel {
    pub id: VesselId,
    pub name: String,
    pub ship: Ship,
    pub vessel: Option<FlightVessel>,
    pub maneuver_nodes: Vec<ManeuverNode>,
    #[serde(default)]
    pub is_debris: bool,
}

/// Metadata about a save file (for the load game UI)
#[derive(Debug, Clone)]
pub struct SaveFileInfo {
    pub name: String,
    pub save_id: String,
    pub vessel_count: usize,
    pub simulation_time: f64,
    pub modified: std::time::SystemTime,
}

/// Metadata about a quicksave file
pub struct QuicksaveInfo {
    pub filename: String,
    pub index: u32,
    pub simulation_time: f64,
    pub modified: std::time::SystemTime,
}

impl SaveGame {
    /// Snapshot the current game state into a SaveGame.
    /// The active vessel is put on-rails before saving.
    pub fn from_game(
        game: &Game,
        save_name: &str,
    ) -> Self {
        let mut vessels = Vec::new();

        // Save the active vessel (put on rails for consistent state)
        let mut active_ship = game.flight.ship.clone();
        active_ship.enter_rails_mode(&game.solar_system);

        vessels.push(SavedVessel {
            id: game.flight.active_vessel_id,
            name: game.flight.active_vessel_name.clone(),
            ship: active_ship,
            vessel: game.flight.vessel.clone(),
            maneuver_nodes: game.flight.active_maneuver_nodes.clone(),
            is_debris: false,
        });

        // Save all inactive vessels
        for tracked in &game.flight.inactive_vessels {
            vessels.push(SavedVessel {
                id: tracked.id,
                name: tracked.name.clone(),
                ship: tracked.ship.clone(),
                vessel: tracked.vessel.clone(),
                maneuver_nodes: tracked.maneuver_nodes.clone(),
                is_debris: tracked.is_debris,
            });
        }

        // Collect blueprints
        let blueprints: Vec<VesselBlueprint> = game.blueprints
            .all_blueprints()
            .into_iter()
            .cloned()
            .collect();

        SaveGame {
            version: SAVE_VERSION,
            name: save_name.to_string(),
            simulation_time: game.time(),
            vessels,
            next_vessel_id: game.flight.next_vessel_id,
            debris_counter: game.flight.debris_counter,
            blueprints,
            editor_vessel_name: game.editor.vessel_name.clone(),
            colonies: game.colony_manager.clone(),
            company: game.company.clone(),
            science: game.science.clone(),
            tech_unlocked: game.tech_tree.unlocked.clone(),
            tech_line_tiers: game.tech_tree.line_tiers.clone(),
            notifications: game.notifications.clone(),
            editor_blueprint: game.editor.to_blueprint(&game.part_definitions).ok(),
        }
    }

    /// Write this save game to disk as `data/saves/{name}/save.ron`.
    pub fn write_to_file(&self) -> Result<(), String> {
        let save_id = sanitize_save_name(&self.name);
        let dir = PathBuf::from(SAVE_DIR).join(&save_id);
        if !dir.exists() {
            fs::create_dir_all(&dir)
                .map_err(|e| format!("Failed to create save directory: {}", e))?;
        }

        let path = dir.join("save.ron");

        let content = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("Failed to serialize save game: {}", e))?;

        atomic_write(&path, &content)?;

        log::info!("Saved game '{}' to {:?}", self.name, path);
        Ok(())
    }

    /// Write a quicksave to `data/saves/{name}/quicksave_{N}.ron`.
    /// Returns the quicksave index.
    pub fn write_quicksave(&self) -> Result<u32, String> {
        let save_id = sanitize_save_name(&self.name);
        let dir = PathBuf::from(SAVE_DIR).join(&save_id);
        if !dir.exists() {
            fs::create_dir_all(&dir)
                .map_err(|e| format!("Failed to create save directory: {}", e))?;
        }

        // Find highest existing quicksave index
        let next_index = next_quicksave_index(&dir);

        let path = dir.join(format!("quicksave_{}.ron", next_index));
        let content = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("Failed to serialize quicksave: {}", e))?;

        atomic_write(&path, &content)?;

        log::info!("Quicksaved '{}' as quicksave_{}", self.name, next_index);
        Ok(next_index)
    }

    /// Write a launch save to `data/saves/{name}/launch.ron`.
    /// Overwrites any existing launch save.
    pub fn write_launch_save(&self) -> Result<(), String> {
        let save_id = sanitize_save_name(&self.name);
        let dir = PathBuf::from(SAVE_DIR).join(&save_id);
        if !dir.exists() {
            fs::create_dir_all(&dir)
                .map_err(|e| format!("Failed to create save directory: {}", e))?;
        }

        let path = dir.join("launch.ron");
        let content = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("Failed to serialize launch save: {}", e))?;

        atomic_write(&path, &content)?;

        log::info!("Saved launch state for '{}'", self.name);
        Ok(())
    }

    /// Load a launch save from `data/saves/{save_name}/launch.ron`.
    pub fn load_launch_save(save_name: &str) -> Result<Self, String> {
        let save_id = sanitize_save_name(save_name);
        let path = PathBuf::from(SAVE_DIR).join(&save_id).join("launch.ron");

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read launch save: {}", e))?;

        let save: SaveGame = ron::from_str(&content)
            .map_err(|e| format!("Failed to parse launch save: {}", e))?;

        if save.version > SAVE_VERSION {
            return Err(format!(
                "Save version too new: max supported {}, got {}",
                SAVE_VERSION, save.version
            ));
        }

        log::info!("Loaded launch save for '{}'", save.name);
        Ok(save)
    }

    /// Load a save game from `data/saves/{save_id}/save.ron`.
    /// Falls back to legacy flat file `data/saves/{save_id}.ron`.
    pub fn load_from_file(save_id: &str) -> Result<Self, String> {
        // Try folder-based first
        let folder_path = PathBuf::from(SAVE_DIR).join(save_id).join("save.ron");
        let path = if folder_path.exists() {
            folder_path
        } else {
            // Legacy fallback: flat .ron file
            let legacy_path = PathBuf::from(SAVE_DIR).join(format!("{}.ron", save_id));
            if legacy_path.exists() {
                legacy_path
            } else {
                return Err(format!("Save '{}' not found", save_id));
            }
        };

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read save file: {}", e))?;

        let save: SaveGame = ron::from_str(&content)
            .map_err(|e| format!("Failed to parse save file: {}", e))?;

        if save.version > SAVE_VERSION {
            return Err(format!(
                "Save version too new: max supported {}, got {}",
                SAVE_VERSION, save.version
            ));
        }

        log::info!("Loaded save game '{}' ({} vessels)", save.name, save.vessels.len());
        Ok(save)
    }

    /// Load a quicksave from `data/saves/{save_name}/quicksave_filename`.
    pub fn load_quicksave(save_name: &str, qs_filename: &str) -> Result<Self, String> {
        let save_id = sanitize_save_name(save_name);
        let path = PathBuf::from(SAVE_DIR).join(&save_id).join(qs_filename);

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read quicksave: {}", e))?;

        let save: SaveGame = ron::from_str(&content)
            .map_err(|e| format!("Failed to parse quicksave: {}", e))?;

        if save.version > SAVE_VERSION {
            return Err(format!(
                "Save version too new: max supported {}, got {}",
                SAVE_VERSION, save.version
            ));
        }

        log::info!("Loaded quicksave '{}' from {}", save.name, qs_filename);
        Ok(save)
    }

    /// List all save files in the saves directory.
    /// Finds folder-based saves (with save.ron) and legacy flat .ron files.
    /// Folders take priority over legacy files with the same name.
    pub fn list_saves() -> Vec<SaveFileInfo> {
        let dir = Path::new(SAVE_DIR);
        if !dir.exists() {
            return Vec::new();
        }

        let mut saves = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return Vec::new(),
        };

        // First pass: folder-based saves (priority)
        let entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        for entry in &entries {
            let path = entry.path();
            if path.is_dir() {
                let save_path = path.join("save.ron");
                if save_path.exists() {
                    let save_id = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    let modified = fs::metadata(&save_path)
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::UNIX_EPOCH);

                    match fs::read_to_string(&save_path) {
                        Ok(content) => {
                            match ron::from_str::<SaveGame>(&content) {
                                Ok(save) => {
                                    seen_ids.insert(save_id.clone());
                                    saves.push(SaveFileInfo {
                                        name: save.name,
                                        save_id,
                                        vessel_count: save.vessels.len(),
                                        simulation_time: save.simulation_time,
                                        modified,
                                    });
                                }
                                Err(e) => {
                                    log::error!("Save file {:?} is corrupted and cannot be loaded: {}", save_path, e);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to read save file {:?}: {}", save_path, e);
                        }
                    }
                }
            }
        }

        // Second pass: legacy flat .ron files (only if not already found as folder)
        for entry in &entries {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "ron") {
                let save_id = path.file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                if seen_ids.contains(&save_id) {
                    continue;
                }

                let modified = entry.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);

                match fs::read_to_string(&path) {
                    Ok(content) => {
                        match ron::from_str::<SaveGame>(&content) {
                            Ok(save) => {
                                saves.push(SaveFileInfo {
                                    name: save.name,
                                    save_id,
                                    vessel_count: save.vessels.len(),
                                    simulation_time: save.simulation_time,
                                    modified,
                                });
                            }
                            Err(e) => {
                                log::error!("Save file {:?} is corrupted and cannot be loaded: {}", path, e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read save file {:?}: {}", path, e);
                    }
                }
            }
        }

        // Sort by modification time, most recent first
        saves.sort_by(|a, b| b.modified.cmp(&a.modified));
        saves
    }

    /// List all quicksaves for a given save name.
    /// Returns sorted by index descending (newest first).
    pub fn list_quicksaves(save_name: &str) -> Vec<QuicksaveInfo> {
        let save_id = sanitize_save_name(save_name);
        let dir = PathBuf::from(SAVE_DIR).join(&save_id);
        if !dir.exists() {
            return Vec::new();
        }

        let mut quicksaves = Vec::new();

        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => return Vec::new(),
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Match quicksave_N.ron pattern
            if let Some(index) = parse_quicksave_index(&filename) {
                let modified = entry.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);

                // Read simulation_time from the file
                let simulation_time = if let Ok(content) = fs::read_to_string(&path) {
                    ron::from_str::<SaveGame>(&content)
                        .map(|s| s.simulation_time)
                        .unwrap_or(0.0)
                } else {
                    0.0
                };

                quicksaves.push(QuicksaveInfo {
                    filename,
                    index,
                    simulation_time,
                    modified,
                });
            }
        }

        // Sort by index descending (newest first)
        quicksaves.sort_by(|a, b| b.index.cmp(&a.index));
        quicksaves
    }

    /// Restore this save game's state into the given Game.
    /// Active vessel's maneuver nodes are loaded into `game.flight.active_maneuver_nodes`.
    pub fn restore_to_game(self, game: &mut Game) {
        // Restore simulation time
        game.solar_system.time = self.simulation_time;

        // Restore editor vessel name
        game.editor.vessel_name = self.editor_vessel_name;

        // Merge blueprints
        game.blueprints.merge_blueprints(self.blueprints);

        // Restore vessels
        let mut vessels_iter = self.vessels.into_iter();

        // First vessel is the active one
        if let Some(active) = vessels_iter.next() {
            game.flight.ship = active.ship;
            game.flight.vessel = active.vessel;
            game.flight.active_vessel_id = active.id;
            game.flight.active_vessel_name = active.name;
            game.flight.active_maneuver_nodes = active.maneuver_nodes;
        }

        // Remaining are inactive
        game.flight.inactive_vessels = vessels_iter
            .map(|sv| TrackedVessel {
                id: sv.id,
                name: sv.name,
                ship: sv.ship,
                vessel: sv.vessel,
                maneuver_nodes: sv.maneuver_nodes,
                is_debris: sv.is_debris,
            })
            .collect();

        game.flight.next_vessel_id = self.next_vessel_id;
        game.flight.debris_counter = self.debris_counter;

        // Restore colony state
        game.colony_manager = self.colonies;
        game.company = self.company;
        game.science = self.science;
        game.tech_tree.apply_save_state(self.tech_unlocked, self.tech_line_tiers);
        game.notifications = self.notifications;
    }
}

/// Parse "quicksave_N.ron" -> Some(N)
fn parse_quicksave_index(filename: &str) -> Option<u32> {
    let stem = filename.strip_suffix(".ron")?;
    let index_str = stem.strip_prefix("quicksave_")?;
    index_str.parse().ok()
}

/// Find the next quicksave index by scanning existing files.
fn next_quicksave_index(dir: &Path) -> u32 {
    let mut max_index = 0u32;

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let filename = entry.file_name();
            let filename = filename.to_str().unwrap_or("");
            if let Some(index) = parse_quicksave_index(filename) {
                max_index = max_index.max(index);
            }
        }
    }

    max_index + 1
}

fn sanitize_save_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
