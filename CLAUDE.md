# Sunscatter

## Architecture

- **Solar system bodies** are hardcoded in `src/bodies.rs` using real-world names (Sun, Earth, Moon, Mars, etc.) and real-world physics values. The `data/bodies/` RON files are stale/unused.
- **Earth** is body index 3 and is the home world where ships spawn.

## Terminology

- **Ship view**: Zoomed in close enough that the ship's triangle icon is invisible.
- **Map view**: Zoomed out enough that the ship's triangle icon is visible.
