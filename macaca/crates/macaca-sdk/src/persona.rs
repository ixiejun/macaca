//! Agent persona system — OpenClaw-inspired identity via Markdown files.
//!
//! A persona is defined by up to 7 Markdown files in a directory:
//! - `BOOTSTRAP.md` — Initial boot instructions, loaded first
//! - `IDENTITY.md` — Core identity, role, and purpose
//! - `AGENTS.md` — Multi-agent coordination rules
//! - `SOUL.md` — Personality, values, communication style
//! - `USER.md` — User context and preferences
//! - `TOOLS.md` — Tool usage guidelines
//! - `HEARTBEAT.md` — Periodic reminders / operational constraints

use std::path::Path;

use macaca_proto::MacacaResult;

/// Known persona file names in load order.
const PERSONA_FILES: &[(&str, &str)] = &[
    ("BOOTSTRAP.md", "Bootstrap"),
    ("IDENTITY.md", "Identity"),
    ("AGENTS.md", "Agents"),
    ("SOUL.md", "Soul"),
    ("USER.md", "User"),
    ("TOOLS.md", "Tools"),
    ("HEARTBEAT.md", "Heartbeat"),
];

/// An agent's persona, assembled from Markdown files.
#[derive(Debug, Clone, Default)]
pub struct AgentPersona {
    /// Initial boot instructions.
    pub bootstrap: Option<String>,
    /// Core identity and purpose.
    pub identity: Option<String>,
    /// Multi-agent coordination rules.
    pub agents: Option<String>,
    /// Personality and communication style.
    pub soul: Option<String>,
    /// User context and preferences.
    pub user: Option<String>,
    /// Tool usage guidelines.
    pub tools: Option<String>,
    /// Periodic reminders / operational constraints.
    pub heartbeat: Option<String>,
}

impl AgentPersona {
    /// Create an empty persona.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load persona files from a directory.
    ///
    /// Reads each known Markdown file if it exists, silently skipping
    /// missing files. Returns an error only on I/O failures (permission
    /// denied, etc.), not on missing files.
    pub async fn load_from_directory(dir: impl AsRef<Path>) -> MacacaResult<Self> {
        let dir = dir.as_ref();
        let mut persona = Self::new();

        for &(filename, _section) in PERSONA_FILES {
            let path = dir.join(filename);
            if !path.exists() {
                continue;
            }

            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(macaca_proto::MacacaError::Io)?;

            let content = content.trim().to_string();
            if content.is_empty() {
                continue;
            }

            match filename {
                "BOOTSTRAP.md" => persona.bootstrap = Some(content),
                "IDENTITY.md" => persona.identity = Some(content),
                "AGENTS.md" => persona.agents = Some(content),
                "SOUL.md" => persona.soul = Some(content),
                "USER.md" => persona.user = Some(content),
                "TOOLS.md" => persona.tools = Some(content),
                "HEARTBEAT.md" => persona.heartbeat = Some(content),
                _ => {}
            }
        }

        Ok(persona)
    }

    /// Whether any persona section is present.
    pub fn is_empty(&self) -> bool {
        self.bootstrap.is_none()
            && self.identity.is_none()
            && self.agents.is_none()
            && self.soul.is_none()
            && self.user.is_none()
            && self.tools.is_none()
            && self.heartbeat.is_none()
    }

    /// Number of loaded sections.
    pub fn section_count(&self) -> usize {
        let sections: [&Option<String>; 7] = [
            &self.bootstrap,
            &self.identity,
            &self.agents,
            &self.soul,
            &self.user,
            &self.tools,
            &self.heartbeat,
        ];
        sections.iter().filter(|s| s.is_some()).count()
    }

    /// Convert the persona into a system prompt.
    ///
    /// Sections are concatenated in order, each wrapped with a header.
    /// If `base_prompt` is provided, it is prepended before persona sections.
    ///
    /// ### Composer integration
    /// When `ContextConfig.agent_profile` is enabled, prefer
    /// [`Self::to_system_prompt_delegating_profile_files`] for Markdown files that duplicate the
    /// composer's profile injection, otherwise identical sections appear twice.
    pub fn to_system_prompt(&self, base_prompt: Option<&str>) -> String {
        let mut parts: Vec<String> = Vec::new();

        if let Some(base) = base_prompt {
            if !base.is_empty() {
                parts.push(base.to_string());
            }
        }

        let sections: &[(&str, &Option<String>)] = &[
            ("Bootstrap", &self.bootstrap),
            ("Identity", &self.identity),
            ("Agents", &self.agents),
            ("Soul", &self.soul),
            ("User", &self.user),
            ("Tools", &self.tools),
            ("Heartbeat", &self.heartbeat),
        ];

        for &(label, content) in sections {
            if let Some(text) = content {
                parts.push(format!("## {label}\n\n{text}"));
            }
        }

        parts.join("\n\n")
    }

    /// Emit only sections that are **not** covered by the context composer's profile provider.
    ///
    /// Markdown that maps 1:1 to composer profile files (`IDENTITY.md`, `TOOLS.md`, …) should
    /// ride the composer pipeline for auditability; today the only persona file that stays here is
    /// `BOOTSTRAP.md`, which is outside that manifest.
    pub fn to_system_prompt_delegating_profile_files(&self, base_prompt: Option<&str>) -> String {
        let mut parts: Vec<String> = Vec::new();

        if let Some(base) = base_prompt {
            if !base.is_empty() {
                parts.push(base.to_string());
            }
        }

        if let Some(ref text) = self.bootstrap {
            parts.push(format!("## Bootstrap\n\n{text}"));
        }

        parts.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[test]
    fn empty_persona() {
        let persona = AgentPersona::new();
        assert!(persona.is_empty());
        assert_eq!(persona.section_count(), 0);
    }

    #[test]
    fn system_prompt_no_sections() {
        let persona = AgentPersona::new();
        let prompt = persona.to_system_prompt(None);
        assert!(prompt.is_empty());
    }

    #[test]
    fn system_prompt_with_base_only() {
        let persona = AgentPersona::new();
        let prompt = persona.to_system_prompt(Some("You are a helpful agent."));
        assert_eq!(prompt, "You are a helpful agent.");
    }

    #[test]
    fn system_prompt_with_sections() {
        let persona = AgentPersona {
            identity: Some("I am Agent X.".into()),
            soul: Some("I am calm and precise.".into()),
            ..Default::default()
        };
        let prompt = persona.to_system_prompt(Some("Base instructions."));
        assert!(prompt.contains("Base instructions."));
        assert!(prompt.contains("## Identity\n\nI am Agent X."));
        assert!(prompt.contains("## Soul\n\nI am calm and precise."));
        assert!(!prompt.contains("## Bootstrap"));
        assert_eq!(persona.section_count(), 2);
        assert!(!persona.is_empty());
    }

    #[test]
    fn delegating_prompt_leaves_only_bootstrap() {
        let persona = AgentPersona {
            bootstrap: Some("boot".into()),
            identity: Some("id".into()),
            tools: Some("tools".into()),
            ..Default::default()
        };
        let prompt = persona.to_system_prompt_delegating_profile_files(None);
        assert!(prompt.contains("Bootstrap"));
        assert!(!prompt.contains("Identity"));
        assert!(!prompt.contains("Tools"));
    }

    #[test]
    fn system_prompt_section_order() {
        let persona = AgentPersona {
            heartbeat: Some("Stay focused.".into()),
            bootstrap: Some("Boot up.".into()),
            ..Default::default()
        };
        let prompt = persona.to_system_prompt(None);
        let boot_pos = prompt.find("## Bootstrap").unwrap();
        let heart_pos = prompt.find("## Heartbeat").unwrap();
        assert!(
            boot_pos < heart_pos,
            "Bootstrap should come before Heartbeat"
        );
    }

    #[tokio::test]
    async fn load_from_directory_full() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("BOOTSTRAP.md"), "Boot instructions")
            .await
            .unwrap();
        fs::write(dir.path().join("IDENTITY.md"), "I am agent X")
            .await
            .unwrap();
        fs::write(dir.path().join("SOUL.md"), "Kind and precise")
            .await
            .unwrap();

        let persona = AgentPersona::load_from_directory(dir.path()).await.unwrap();
        assert_eq!(persona.bootstrap.as_deref(), Some("Boot instructions"));
        assert_eq!(persona.identity.as_deref(), Some("I am agent X"));
        assert_eq!(persona.soul.as_deref(), Some("Kind and precise"));
        assert!(persona.agents.is_none());
        assert_eq!(persona.section_count(), 3);
    }

    #[tokio::test]
    async fn load_skips_empty_files() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("IDENTITY.md"), "I exist")
            .await
            .unwrap();
        fs::write(dir.path().join("SOUL.md"), "   \n  ") // whitespace only
            .await
            .unwrap();

        let persona = AgentPersona::load_from_directory(dir.path()).await.unwrap();
        assert!(persona.identity.is_some());
        assert!(persona.soul.is_none()); // empty after trim
        assert_eq!(persona.section_count(), 1);
    }

    #[tokio::test]
    async fn load_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let persona = AgentPersona::load_from_directory(dir.path()).await.unwrap();
        assert!(persona.is_empty());
    }

    #[tokio::test]
    async fn load_ignores_unknown_files() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("IDENTITY.md"), "I am here")
            .await
            .unwrap();
        fs::write(dir.path().join("README.md"), "Not a persona file")
            .await
            .unwrap();
        fs::write(dir.path().join("random.txt"), "Ignore me")
            .await
            .unwrap();

        let persona = AgentPersona::load_from_directory(dir.path()).await.unwrap();
        assert_eq!(persona.section_count(), 1);
        assert!(persona.identity.is_some());
    }
}
