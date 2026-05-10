use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CategoryError {
    #[error("category name cannot be empty")]
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PrimaryCategory(String);

impl PrimaryCategory {
    pub fn new(value: impl Into<String>) -> Result<Self, CategoryError> {
        let value = value.into().trim().to_string();
        if value.is_empty() {
            return Err(CategoryError::Empty);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GenreFacet(String);

impl GenreFacet {
    pub fn new(value: impl Into<String>) -> Result<Self, CategoryError> {
        let value = value.into().trim().to_string();
        if value.is_empty() {
            return Err(CategoryError::Empty);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_category_names() {
        let category = PrimaryCategory::new("  Simulation  ").unwrap();
        assert_eq!(category.as_str(), "Simulation");
    }

    #[test]
    fn rejects_empty_category_names() {
        assert!(PrimaryCategory::new(" ").is_err());
    }

    #[test]
    fn supports_mixed_genre_facets() {
        let facets = vec![
            GenreFacet::new("Simulation").unwrap(),
            GenreFacet::new("Strategy").unwrap(),
        ];
        assert_eq!(facets[0].as_str(), "Simulation");
        assert_eq!(facets[1].as_str(), "Strategy");
    }
}
