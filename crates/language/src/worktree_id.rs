use std::fmt;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord, serde::Serialize)]
pub struct WorktreeId(usize);

impl From<WorktreeId> for usize {
    fn from(value: WorktreeId) -> Self {
        value.0
    }
}

impl WorktreeId {
    pub const fn from_usize(handle_id: usize) -> Self {
        Self(handle_id)
    }

    pub const fn to_usize(self) -> usize {
        self.0
    }
}

impl fmt::Display for WorktreeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::WorktreeId;

    #[test]
    fn worktree_id_round_trips_and_displays() {
        let id = WorktreeId::from_usize(7);
        assert_eq!(id.to_usize(), 7);
        assert_eq!(usize::from(id), 7);
        assert_eq!(id.to_string(), "7");
    }
}
