use tenui_core::Rect;

/// Directional vector for spatial navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// An interactive focusable node with spatial coordinates and an optional focus scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusNode {
    pub id: u64,
    pub rect: Rect,
    pub scope_id: Option<u64>,
}

impl FocusNode {
    pub fn new(id: u64, rect: Rect) -> Self {
        Self {
            id,
            rect,
            scope_id: None,
        }
    }

    pub fn with_scope(mut self, scope_id: u64) -> Self {
        self.scope_id = Some(scope_id);
        self
    }

    pub fn center(&self) -> (f32, f32) {
        (
            self.rect.x as f32 + (self.rect.width as f32 / 2.0),
            self.rect.y as f32 + (self.rect.height as f32 / 2.0),
        )
    }
}

/// Aspect-Ratio-Weighted WICG Spatial Navigation Engine.
///
/// Weight factor k ≈ 2.0 cancels out cell tallness in anisotropic terminal grids.
#[derive(Debug, Clone)]
pub struct SpatialNav {
    pub k_aspect: f32,
    pub is_rtl: bool,
}

impl Default for SpatialNav {
    fn default() -> Self {
        Self {
            k_aspect: 2.0,
            is_rtl: false,
        }
    }
}

impl SpatialNav {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_rtl(mut self, is_rtl: bool) -> Self {
        self.is_rtl = is_rtl;
        self
    }

    /// Selects the best candidate node in the specified direction from the current focused node.
    ///
    /// When `active_scope` is `Some`, acts as a Focus Scope Barrier, restricting navigation
    /// exclusively to nodes belonging to that scope (e.g. within an active modal).
    /// When `is_rtl` is active, horizontal distance vectors mirror directionally across the y-axis.
    pub fn navigate(
        &self,
        current_id: u64,
        direction: Direction,
        nodes: &[FocusNode],
        active_scope: Option<u64>,
    ) -> Option<u64> {
        let current = nodes.iter().find(|n| n.id == current_id)?;
        let (cx, cy) = current.center();

        let effective_dir = if self.is_rtl {
            match direction {
                Direction::Left => Direction::Right,
                Direction::Right => Direction::Left,
                other => other,
            }
        } else {
            direction
        };

        let mut best_id = None;
        let mut best_dist = f32::MAX;

        for candidate in nodes {
            if candidate.id == current_id {
                continue;
            }

            // Focus Scope Barrier: restrict candidate evaluation to active modal
            if let Some(scope) = active_scope {
                if candidate.scope_id != Some(scope) {
                    continue;
                }
            } else if candidate.scope_id.is_some() {
                // If no active scope, ignore modal internal elements
                continue;
            }

            let (tx, ty) = candidate.center();
            let dx = tx - cx;
            let dy = ty - cy;

            // Directional half-plane check
            let in_direction = match effective_dir {
                Direction::Right => dx > 0.0,
                Direction::Left => dx < 0.0,
                Direction::Down => dy > 0.0,
                Direction::Up => dy < 0.0,
            };

            if !in_direction {
                continue;
            }

            // Cross-axis overlap check (primary projection beam between split panes)
            let overlaps_cross_axis = match effective_dir {
                Direction::Right | Direction::Left => {
                    candidate.rect.y < current.rect.bottom() && candidate.rect.bottom() > current.rect.y
                }
                Direction::Down | Direction::Up => {
                    candidate.rect.x < current.rect.right() && candidate.rect.right() > current.rect.x
                }
            };

            // Directional cone test
            let in_cone = match effective_dir {
                Direction::Right => (dy.abs() * self.k_aspect) <= (dx * 2.5),
                Direction::Left => (dy.abs() * self.k_aspect) <= (-dx * 2.5),
                Direction::Down => dx.abs() <= (dy * self.k_aspect * 2.5),
                Direction::Up => dx.abs() <= (-dy * self.k_aspect * 2.5),
            };

            if !overlaps_cross_axis && !in_cone {
                continue;
            }

            // Aspect-ratio weighted distance metric:
            // D = dx^2 + (k * dy)^2, with direct cross-axis overlap favored
            let weight = if overlaps_cross_axis { 0.8 } else { 1.0 };
            let dist = (dx * dx + (self.k_aspect * dy) * (self.k_aspect * dy)) * weight;

            if dist < best_dist {
                best_dist = dist;
                best_id = Some(candidate.id);
            }
        }

        best_id
    }
}
