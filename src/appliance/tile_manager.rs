use crate::config::{StationConfig, StationTilePosition, StationWorkerConfig};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TilePlacement {
    pub worker_id: String,
    pub tile_position: StationTilePosition,
    pub window_origin_x: i32,
    pub window_origin_y: i32,
    pub viewport_width: u32,
    pub viewport_height: u32,
}

#[derive(Debug, Clone)]
pub struct TileManager {
    placements: HashMap<String, TilePlacement>,
}

impl TileManager {
    pub fn from_station_config(config: &StationConfig) -> Result<Self> {
        let mut placements = HashMap::new();

        for worker in &config.workers {
            let placement = TilePlacement {
                worker_id: worker.id.clone(),
                tile_position: worker.tile_position,
                window_origin_x: worker.managed_browser.window_origin_x,
                window_origin_y: worker.managed_browser.window_origin_y,
                viewport_width: worker.managed_browser.viewport_width,
                viewport_height: worker.managed_browser.viewport_height,
            };

            if placements.insert(worker.id.clone(), placement).is_some() {
                return Err(anyhow!(
                    "duplicate tile placement registered for {}",
                    worker.id
                ));
            }
        }

        let manager = Self { placements };
        manager.validate_layout(config)?;
        Ok(manager)
    }

    pub fn placement_for_worker(&self, worker_id: &str) -> Option<&TilePlacement> {
        self.placements.get(worker_id)
    }

    pub fn placements(&self) -> impl Iterator<Item = &TilePlacement> {
        self.placements.values()
    }

    fn validate_layout(&self, config: &StationConfig) -> Result<()> {
        for worker in &config.workers {
            let placement = self
                .placement_for_worker(&worker.id)
                .ok_or_else(|| anyhow!("missing tile placement for {}", worker.id))?;

            if placement.viewport_width == 0 || placement.viewport_height == 0 {
                return Err(anyhow!(
                    "worker {} has invalid viewport dimensions {}x{}",
                    worker.id,
                    placement.viewport_width,
                    placement.viewport_height
                ));
            }

            match worker.tile_position {
                StationTilePosition::Left if placement.window_origin_x != 0 => {
                    return Err(anyhow!("left tile worker {} must start at x=0", worker.id));
                }
                StationTilePosition::Right if placement.window_origin_x < 0 => {
                    return Err(anyhow!(
                        "right tile worker {} must have a non-negative x origin",
                        worker.id
                    ));
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn build_expected_placement(&self, worker: &StationWorkerConfig) -> Result<TilePlacement> {
        self.placement_for_worker(&worker.id)
            .cloned()
            .ok_or_else(|| anyhow!("missing tile placement for {}", worker.id))
    }
}
