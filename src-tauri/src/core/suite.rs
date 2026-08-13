use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const SUITE_LIFECYCLES: &[&str] = &["experimental", "active", "deprecated", "archived"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuiteMember {
    pub skill_id: String,
    pub required: bool,
    pub role: Option<String>,
    pub version_requirement: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuiteInput {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub category: String,
    pub lifecycle: String,
    pub tags: Vec<String>,
    pub members: Vec<SuiteMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuiteRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub category: String,
    pub lifecycle: String,
    pub tags: Vec<String>,
    pub members: Vec<SuiteMember>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl SuiteInput {
    pub fn validate_and_normalize(mut self) -> Result<Self> {
        self.id = optional_text(self.id);
        if let Some(id) = &self.id {
            if id == "."
                || id == ".."
                || !id
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
            {
                bail!("invalid suite id: {id}");
            }
        }
        self.name = required_text("suite name", self.name)?;
        self.description = optional_text(self.description);
        self.version = optional_text(self.version);
        self.category = required_text("suite category", self.category)?;
        if !SUITE_LIFECYCLES.contains(&self.lifecycle.as_str()) {
            bail!("invalid suite lifecycle: {}", self.lifecycle);
        }

        self.tags = self
            .tags
            .into_iter()
            .filter_map(|tag| optional_text(Some(tag)))
            .collect();
        self.tags.sort();
        self.tags.dedup();

        let mut seen = HashSet::new();
        for (index, member) in self.members.iter_mut().enumerate() {
            member.skill_id = required_text(
                "suite member skill_id",
                std::mem::take(&mut member.skill_id),
            )?;
            if !seen.insert(member.skill_id.clone()) {
                bail!("duplicate suite member: {}", member.skill_id);
            }
            member.role = optional_text(member.role.take());
            member.version_requirement = optional_text(member.version_requirement.take());
            member.sort_order = index as i32;
        }
        if self.members.is_empty() {
            bail!("a suite must contain at least one skill");
        }
        Ok(self)
    }
}

fn required_text(field: &str, value: String) -> Result<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        bail!("{field} cannot be empty");
    }
    Ok(value)
}

fn optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty()).then_some(value)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_input_normalizes_tags_and_member_order() {
        let input = SuiteInput {
            id: None,
            name: " Scholar ".into(),
            description: None,
            version: Some(" 1.0 ".into()),
            category: " research ".into(),
            lifecycle: "active".into(),
            tags: vec!["paper".into(), " paper ".into()],
            members: vec![SuiteMember {
                skill_id: " literature-review ".into(),
                required: true,
                role: Some(" entry ".into()),
                version_requirement: None,
                sort_order: 99,
            }],
        }
        .validate_and_normalize()
        .unwrap();
        assert_eq!(input.name, "Scholar");
        assert_eq!(input.tags, vec!["paper"]);
        assert_eq!(input.members[0].sort_order, 0);
    }

    #[test]
    fn suite_id_cannot_escape_registry_directory() {
        let input = SuiteInput {
            id: Some("../outside".into()),
            name: "Unsafe".into(),
            description: None,
            version: None,
            category: "test".into(),
            lifecycle: "active".into(),
            tags: Vec::new(),
            members: vec![SuiteMember {
                skill_id: "skill-1".into(),
                required: true,
                role: None,
                version_requirement: None,
                sort_order: 0,
            }],
        };
        assert!(input.validate_and_normalize().is_err());
    }
}
