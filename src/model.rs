use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type TaskId = u32;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    A,
    B,
    C,
}

impl Default for Priority {
    fn default() -> Self {
        Self::B
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::B => write!(f, "B"),
            Self::C => write!(f, "C"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "A" => Ok(Self::A),
            "B" => Ok(Self::B),
            "C" => Ok(Self::C),
            _ => Err(format!("invalid priority '{s}', expected A, B, or C")),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Active,
    Completed,
    SoftDeleted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub text: String,
    pub priority: Priority,
    pub due: DateTime<Utc>,
    pub est_secs: i64,
    pub status: Status,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_default_is_b() {
        assert_eq!(Priority::default(), Priority::B);
    }

    #[test]
    fn priority_from_str_a() {
        assert_eq!("a".parse::<Priority>().unwrap(), Priority::A);
    }

    #[test]
    fn priority_from_str_b() {
        assert_eq!("B".parse::<Priority>().unwrap(), Priority::B);
    }

    #[test]
    fn priority_from_str_c() {
        assert_eq!("C".parse::<Priority>().unwrap(), Priority::C);
    }

    #[test]
    fn priority_from_str_invalid() {
        assert!("X".parse::<Priority>().is_err());
    }

    #[test]
    fn priority_display() {
        assert_eq!(Priority::A.to_string(), "A");
        assert_eq!(Priority::B.to_string(), "B");
        assert_eq!(Priority::C.to_string(), "C");
    }

    #[test]
    fn task_serde_bincode_roundtrip() {
        use chrono::Timelike;
        let now = Utc::now().with_nanosecond(0).unwrap_or(Utc::now());
        let task = Task {
            id: 1,
            text: "hello".to_string(),
            priority: Priority::A,
            due: now,
            est_secs: 600,
            status: Status::Active,
            created_at: now,
            completed_at: None,
            deleted_at: None,
        };
        let encoded = bincode::serialize(&task).unwrap();
        let decoded: Task = bincode::deserialize(&encoded).unwrap();
        assert_eq!(task, decoded);
    }
}
