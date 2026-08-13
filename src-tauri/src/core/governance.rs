use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub const SKILL_KINDS: &[&str] = &[
    "capability",
    "tool_guide",
    "integration",
    "workflow",
    "governance",
];
pub const PROVENANCE_KINDS: &[&str] = &["unknown", "official", "suite", "third_party", "local"];
pub const LIFECYCLE_STATES: &[&str] = &["experimental", "active", "deprecated", "archived"];
pub const RISK_LEVELS: &[&str] = &["unreviewed", "low", "medium", "high"];
pub const SUPPORT_LEVELS: &[&str] = &["unknown", "supported", "partial", "unsupported"];
pub const DEPENDENCY_TYPES: &[&str] = &["cli", "mcp", "runtime", "service", "skill", "config"];
pub const FILESYSTEM_ACCESS_LEVELS: &[&str] = &["unknown", "none", "read", "write"];
pub const BINARY_ACCESS_LEVELS: &[&str] = &["unknown", "none", "required"];
pub const ACCOUNT_ACCESS_LEVELS: &[&str] = &["unknown", "none", "read", "write"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentSupport {
    pub agent_key: String,
    pub support_level: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillDependency {
    pub dependency_type: String,
    pub name: String,
    pub version_requirement: Option<String>,
    pub required: bool,
    pub check_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillPermissions {
    pub filesystem_access: String,
    pub network_access: String,
    pub command_execution: String,
    pub account_access: String,
    pub secrets_access: String,
}

impl Default for SkillPermissions {
    fn default() -> Self {
        Self {
            filesystem_access: "unknown".to_string(),
            network_access: "unknown".to_string(),
            command_execution: "unknown".to_string(),
            account_access: "unknown".to_string(),
            secrets_access: "unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillGovernanceInput {
    pub skill_id: String,
    pub kind: String,
    pub provenance: String,
    pub owner: Option<String>,
    pub maintainer: Option<String>,
    pub lifecycle: String,
    pub risk_level: String,
    pub supported_agents: Vec<AgentSupport>,
    pub dependencies: Vec<SkillDependency>,
    pub permissions: SkillPermissions,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillGovernanceProfile {
    pub skill_id: String,
    pub kind: String,
    pub provenance: String,
    pub owner: Option<String>,
    pub maintainer: Option<String>,
    pub lifecycle: String,
    pub risk_level: String,
    pub supported_agents: Vec<AgentSupport>,
    pub dependencies: Vec<SkillDependency>,
    pub permissions: SkillPermissions,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl SkillGovernanceProfile {
    pub fn default_for(skill_id: impl Into<String>) -> Self {
        Self {
            skill_id: skill_id.into(),
            kind: "capability".to_string(),
            provenance: "unknown".to_string(),
            owner: None,
            maintainer: None,
            lifecycle: "active".to_string(),
            risk_level: "unreviewed".to_string(),
            supported_agents: Vec::new(),
            dependencies: Vec::new(),
            permissions: SkillPermissions::default(),
            notes: None,
            created_at: 0,
            updated_at: 0,
        }
    }
}

impl From<SkillGovernanceProfile> for SkillGovernanceInput {
    fn from(profile: SkillGovernanceProfile) -> Self {
        Self {
            skill_id: profile.skill_id,
            kind: profile.kind,
            provenance: profile.provenance,
            owner: profile.owner,
            maintainer: profile.maintainer,
            lifecycle: profile.lifecycle,
            risk_level: profile.risk_level,
            supported_agents: profile.supported_agents,
            dependencies: profile.dependencies,
            permissions: profile.permissions,
            notes: profile.notes,
        }
    }
}

impl SkillGovernanceInput {
    pub fn validate_and_normalize(mut self) -> Result<Self> {
        self.skill_id = required_text("skill_id", self.skill_id)?;
        validate_enum("kind", &self.kind, SKILL_KINDS)?;
        validate_enum("provenance", &self.provenance, PROVENANCE_KINDS)?;
        validate_enum("lifecycle", &self.lifecycle, LIFECYCLE_STATES)?;
        validate_enum("risk_level", &self.risk_level, RISK_LEVELS)?;
        validate_enum(
            "permissions.filesystem_access",
            &self.permissions.filesystem_access,
            FILESYSTEM_ACCESS_LEVELS,
        )?;
        validate_enum(
            "permissions.network_access",
            &self.permissions.network_access,
            BINARY_ACCESS_LEVELS,
        )?;
        validate_enum(
            "permissions.command_execution",
            &self.permissions.command_execution,
            BINARY_ACCESS_LEVELS,
        )?;
        validate_enum(
            "permissions.account_access",
            &self.permissions.account_access,
            ACCOUNT_ACCESS_LEVELS,
        )?;
        validate_enum(
            "permissions.secrets_access",
            &self.permissions.secrets_access,
            BINARY_ACCESS_LEVELS,
        )?;

        self.owner = optional_text(self.owner);
        self.maintainer = optional_text(self.maintainer);
        self.notes = optional_text(self.notes);

        for support in &mut self.supported_agents {
            support.agent_key = required_text(
                "supported_agents.agent_key",
                std::mem::take(&mut support.agent_key),
            )?;
            validate_enum(
                "supported_agents.support_level",
                &support.support_level,
                SUPPORT_LEVELS,
            )?;
            support.notes = optional_text(support.notes.take());
        }
        self.supported_agents
            .sort_by(|a, b| a.agent_key.cmp(&b.agent_key));
        self.supported_agents
            .dedup_by(|a, b| a.agent_key == b.agent_key);

        for dependency in &mut self.dependencies {
            validate_enum(
                "dependencies.dependency_type",
                &dependency.dependency_type,
                DEPENDENCY_TYPES,
            )?;
            dependency.name =
                required_text("dependencies.name", std::mem::take(&mut dependency.name))?;
            dependency.version_requirement = optional_text(dependency.version_requirement.take());
            dependency.check_command = optional_text(dependency.check_command.take());
        }
        self.dependencies
            .sort_by(|a, b| (&a.dependency_type, &a.name).cmp(&(&b.dependency_type, &b.name)));
        self.dependencies
            .dedup_by(|a, b| a.dependency_type == b.dependency_type && a.name == b.name);

        Ok(self)
    }
}

fn validate_enum(field: &str, value: &str, allowed: &[&str]) -> Result<()> {
    if !allowed.contains(&value) {
        bail!(
            "Invalid {field} value '{value}'. Expected one of: {}",
            allowed.join(", ")
        );
    }
    Ok(())
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

    fn valid_input() -> SkillGovernanceInput {
        SkillGovernanceInput {
            skill_id: " skill-1 ".to_string(),
            kind: "integration".to_string(),
            provenance: "local".to_string(),
            owner: Some(" owner ".to_string()),
            maintainer: Some("  ".to_string()),
            lifecycle: "active".to_string(),
            risk_level: "medium".to_string(),
            supported_agents: vec![AgentSupport {
                agent_key: "codex".to_string(),
                support_level: "supported".to_string(),
                notes: None,
            }],
            dependencies: vec![SkillDependency {
                dependency_type: "cli".to_string(),
                name: " opencli ".to_string(),
                version_requirement: Some(" >=1.0 ".to_string()),
                required: true,
                check_command: Some(" opencli --version ".to_string()),
            }],
            permissions: SkillPermissions::default(),
            notes: None,
        }
    }

    #[test]
    fn normalizes_governance_input() {
        let input = valid_input().validate_and_normalize().unwrap();
        assert_eq!(input.skill_id, "skill-1");
        assert_eq!(input.owner.as_deref(), Some("owner"));
        assert_eq!(input.maintainer, None);
        assert_eq!(input.dependencies[0].name, "opencli");
        assert_eq!(
            input.dependencies[0].version_requirement.as_deref(),
            Some(">=1.0")
        );
    }

    #[test]
    fn rejects_invalid_kind() {
        let mut input = valid_input();
        input.kind = "suite_member".to_string();
        assert!(input
            .validate_and_normalize()
            .unwrap_err()
            .to_string()
            .contains("Invalid kind"));
    }

    #[test]
    fn rejects_empty_dependency_name() {
        let mut input = valid_input();
        input.dependencies[0].name = "  ".to_string();
        assert!(input
            .validate_and_normalize()
            .unwrap_err()
            .to_string()
            .contains("dependencies.name cannot be empty"));
    }
}
