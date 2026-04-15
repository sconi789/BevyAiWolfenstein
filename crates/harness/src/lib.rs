use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelPlan {
    pub seed: u64,
    pub pieces: Vec<Piece>,
    pub player_start: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Piece {
    Floor { pos: [f32; 3], size: [f32; 3] },
    Wall { pos: [f32; 3], size: [f32; 3] },
    Door { pos: [f32; 3], size: [f32; 3], locked: bool },
    Key { pos: [f32; 3] },
    Light { pos: [f32; 3], intensity: f32, range: f32 },
}

#[derive(Debug, Error)]
pub enum HarnessError {
    #[error("level plan yaml parse failed: {0}")]
    Parse(#[from] serde_yaml::Error),
    #[error("level plan gate failed: {0}")]
    Gate(String),
    #[error("level plan read failed: {0}")]
    Io(#[from] std::io::Error),
}

pub fn read_level_plan_from_path(path: impl AsRef<std::path::Path>) -> Result<LevelPlan, HarnessError> {
    let s = std::fs::read_to_string(path)?;
    let plan: LevelPlan = serde_yaml::from_str(&s)?;
    gate_level_plan(&plan)?;
    Ok(plan)
}

pub fn gate_level_plan(plan: &LevelPlan) -> Result<(), HarnessError> {
    if plan.pieces.is_empty() {
        return Err(HarnessError::Gate("pieces 不能为空".to_string()));
    }
    if plan.pieces.len() > 2000 {
        return Err(HarnessError::Gate("pieces 过多（>2000），灰盒 demo 请先控制规模".to_string()));
    }
    let has_floor = plan.pieces.iter().any(|p| matches!(p, Piece::Floor { .. }));
    if !has_floor {
        return Err(HarnessError::Gate("必须至少包含一个 Floor".to_string()));
    }
    let has_key = plan.pieces.iter().any(|p| matches!(p, Piece::Key { .. }));
    let has_locked_door = plan
        .pieces
        .iter()
        .any(|p| matches!(p, Piece::Door { locked: true, .. }));
    if has_locked_door && !has_key {
        return Err(HarnessError::Gate("存在 locked door 但没有 Key".to_string()));
    }

    let within = |v: [f32; 3], limit: f32| v.into_iter().all(|x| x.is_finite() && x.abs() <= limit);
    if !within(plan.player_start, 10_000.0) {
        return Err(HarnessError::Gate("player_start 超出允许范围或包含 NaN".to_string()));
    }

    for (i, p) in plan.pieces.iter().enumerate() {
        match p {
            Piece::Floor { pos, size } | Piece::Wall { pos, size } | Piece::Door { pos, size, .. } => {
                if !within(*pos, 10_000.0) {
                    return Err(HarnessError::Gate(format!("piece[{i}] pos 非法")));
                }
                if size.iter().any(|x| !x.is_finite() || *x <= 0.0 || *x > 5000.0) {
                    return Err(HarnessError::Gate(format!("piece[{i}] size 非法")));
                }
            }
            Piece::Key { pos } => {
                if !within(*pos, 10_000.0) {
                    return Err(HarnessError::Gate(format!("piece[{i}] key pos 非法")));
                }
            }
            Piece::Light {
                pos,
                intensity,
                range,
            } => {
                if !within(*pos, 10_000.0) {
                    return Err(HarnessError::Gate(format!("piece[{i}] light pos 非法")));
                }
                if !intensity.is_finite() || *intensity <= 0.0 || *intensity > 200_000.0 {
                    return Err(HarnessError::Gate(format!("piece[{i}] light intensity 非法")));
                }
                if !range.is_finite() || *range <= 0.0 || *range > 5000.0 {
                    return Err(HarnessError::Gate(format!("piece[{i}] light range 非法")));
                }
            }
        }
    }

    Ok(())
}

