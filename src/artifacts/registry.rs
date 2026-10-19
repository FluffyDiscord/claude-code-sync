use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CategoryId {
    Settings,
    Memory,
    Skills,
    Agents,
    Commands,
    Rules,
    Hooks,
    Plugins,
    Plans,
    Todos,
    PromptHistory,
    ProjectAttachments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceSpec {
    Files(&'static [&'static str]),
    Dir(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    RawOverwrite,
    UnionJsonl,
    UnionMemoryIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestRoot {
    Artifacts,
    SessionTree,
}

#[derive(Debug)]
pub struct CategoryDescriptor {
    pub id: CategoryId,
    pub name: &'static str,
    pub repo_subdir: &'static str,
    pub source: SourceSpec,
    pub merge: MergeStrategy,
    pub dest: DestRoot,
    pub exclude_extensions: &'static [&'static str],
    pub tokenize_paths: bool,
    pub mirror_deletes: bool,
    pub description: &'static str,
}

pub const ARTIFACTS_SUBDIR: &str = "artifacts";

pub static REGISTRY: &[CategoryDescriptor] = &[
    CategoryDescriptor {
        id: CategoryId::Settings,
        name: "settings",
        repo_subdir: "settings",
        source: SourceSpec::Files(&["settings.json", "keybindings.json"]),
        merge: MergeStrategy::RawOverwrite,
        dest: DestRoot::Artifacts,
        exclude_extensions: &[],
        tokenize_paths: true,
        mirror_deletes: false,
        description: "User settings and keybindings (never settings.local.json)",
    },
    CategoryDescriptor {
        id: CategoryId::Memory,
        name: "memory",
        repo_subdir: "memory",
        source: SourceSpec::Files(&["CLAUDE.md"]),
        merge: MergeStrategy::RawOverwrite,
        dest: DestRoot::Artifacts,
        exclude_extensions: &[],
        tokenize_paths: false,
        mirror_deletes: false,
        description: "Global CLAUDE.md user memory",
    },
    CategoryDescriptor {
        id: CategoryId::Skills,
        name: "skills",
        repo_subdir: "skills",
        source: SourceSpec::Dir("skills"),
        merge: MergeStrategy::RawOverwrite,
        dest: DestRoot::Artifacts,
        exclude_extensions: &[],
        tokenize_paths: false,
        mirror_deletes: true,
        description: "Custom skills",
    },
    CategoryDescriptor {
        id: CategoryId::Agents,
        name: "agents",
        repo_subdir: "agents",
        source: SourceSpec::Dir("agents"),
        merge: MergeStrategy::RawOverwrite,
        dest: DestRoot::Artifacts,
        exclude_extensions: &[],
        tokenize_paths: false,
        mirror_deletes: true,
        description: "Custom subagent definitions",
    },
    CategoryDescriptor {
        id: CategoryId::Commands,
        name: "commands",
        repo_subdir: "commands",
        source: SourceSpec::Dir("commands"),
        merge: MergeStrategy::RawOverwrite,
        dest: DestRoot::Artifacts,
        exclude_extensions: &[],
        tokenize_paths: false,
        mirror_deletes: true,
        description: "Custom slash commands",
    },
    CategoryDescriptor {
        id: CategoryId::Rules,
        name: "rules",
        repo_subdir: "rules",
        source: SourceSpec::Dir("rules"),
        merge: MergeStrategy::RawOverwrite,
        dest: DestRoot::Artifacts,
        exclude_extensions: &[],
        tokenize_paths: false,
        mirror_deletes: true,
        description: "Shared rule files projects import from ~/.claude/rules",
    },
    CategoryDescriptor {
        id: CategoryId::Hooks,
        name: "hooks",
        repo_subdir: "hooks",
        source: SourceSpec::Dir("hooks"),
        merge: MergeStrategy::RawOverwrite,
        dest: DestRoot::Artifacts,
        exclude_extensions: &[],
        tokenize_paths: false,
        mirror_deletes: true,
        description: "Hook scripts settings.json points at (they run automatically)",
    },
    CategoryDescriptor {
        id: CategoryId::Plugins,
        name: "plugins",
        repo_subdir: "plugins",
        source: SourceSpec::Files(&[
            "plugins/installed_plugins.json",
            "plugins/known_marketplaces.json",
        ]),
        merge: MergeStrategy::RawOverwrite,
        dest: DestRoot::Artifacts,
        exclude_extensions: &[],
        tokenize_paths: true,
        mirror_deletes: false,
        description: "Installed-plugin and marketplace manifests (never plugin caches)",
    },
    CategoryDescriptor {
        id: CategoryId::Plans,
        name: "plans",
        repo_subdir: "plans",
        source: SourceSpec::Dir("plans"),
        merge: MergeStrategy::RawOverwrite,
        dest: DestRoot::Artifacts,
        exclude_extensions: &[],
        tokenize_paths: false,
        mirror_deletes: false,
        description: "Plan-mode documents (may contain sensitive prose)",
    },
    CategoryDescriptor {
        id: CategoryId::Todos,
        name: "todos",
        repo_subdir: "todos",
        source: SourceSpec::Dir("todos"),
        merge: MergeStrategy::RawOverwrite,
        dest: DestRoot::Artifacts,
        exclude_extensions: &[],
        tokenize_paths: false,
        mirror_deletes: false,
        description: "Session task lists (changes frequently)",
    },
    CategoryDescriptor {
        id: CategoryId::ProjectAttachments,
        name: "attachments",
        repo_subdir: "",
        source: SourceSpec::Dir("projects"),
        merge: MergeStrategy::UnionMemoryIndex,
        dest: DestRoot::SessionTree,
        exclude_extensions: &["jsonl"],
        tokenize_paths: false,
        mirror_deletes: false,
        description: "Non-transcript files in project dirs (images, PDFs, project memory)",
    },
    CategoryDescriptor {
        id: CategoryId::PromptHistory,
        name: "prompt-history",
        repo_subdir: "prompt-history",
        source: SourceSpec::Files(&["history.jsonl"]),
        merge: MergeStrategy::UnionJsonl,
        dest: DestRoot::Artifacts,
        exclude_extensions: &[],
        tokenize_paths: false,
        mirror_deletes: false,
        description: "Cross-project prompt history (union-merged, grow-only)",
    },
];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactToggles {
    #[serde(default)]
    pub settings: bool,
    #[serde(default)]
    pub memory: bool,
    #[serde(default)]
    pub skills: bool,
    #[serde(default)]
    pub agents: bool,
    #[serde(default)]
    pub commands: bool,
    #[serde(default)]
    pub rules: bool,
    #[serde(default)]
    pub hooks: bool,
    #[serde(default)]
    pub plugins: bool,
    #[serde(default)]
    pub plans: bool,
    #[serde(default)]
    pub todos: bool,
    #[serde(default)]
    pub prompt_history: bool,
}

impl ArtifactToggles {
    pub fn all_enabled() -> Self {
        Self {
            settings: true,
            memory: true,
            skills: true,
            agents: true,
            commands: true,
            rules: true,
            hooks: true,
            plugins: true,
            plans: true,
            todos: true,
            prompt_history: true,
        }
    }

    pub fn all_but_hooks() -> Self {
        Self {
            hooks: false,
            ..Self::all_enabled()
        }
    }

    pub fn any_enabled(&self) -> bool {
        REGISTRY.iter().any(|d| self.is_enabled(d.id))
    }

    pub fn is_enabled(&self, id: CategoryId) -> bool {
        match id {
            CategoryId::Settings => self.settings,
            CategoryId::Memory => self.memory,
            CategoryId::Skills => self.skills,
            CategoryId::Agents => self.agents,
            CategoryId::Commands => self.commands,
            CategoryId::Rules => self.rules,
            CategoryId::Hooks => self.hooks,
            CategoryId::Plugins => self.plugins,
            CategoryId::Plans => self.plans,
            CategoryId::Todos => self.todos,
            CategoryId::PromptHistory => self.prompt_history,
            CategoryId::ProjectAttachments => false,
        }
    }

    pub fn set_enabled(&mut self, id: CategoryId, value: bool) {
        match id {
            CategoryId::Settings => self.settings = value,
            CategoryId::Memory => self.memory = value,
            CategoryId::Skills => self.skills = value,
            CategoryId::Agents => self.agents = value,
            CategoryId::Commands => self.commands = value,
            CategoryId::Rules => self.rules = value,
            CategoryId::Hooks => self.hooks = value,
            CategoryId::Plugins => self.plugins = value,
            CategoryId::Plans => self.plans = value,
            CategoryId::Todos => self.todos = value,
            CategoryId::PromptHistory => self.prompt_history = value,
            CategoryId::ProjectAttachments => {}
        }
    }
}

pub fn find_by_name(name: &str) -> Option<&'static CategoryDescriptor> {
    REGISTRY.iter().find(|d| d.name == name)
}

pub fn toggleable() -> impl Iterator<Item = &'static CategoryDescriptor> {
    REGISTRY
        .iter()
        .filter(|d| d.id != CategoryId::ProjectAttachments)
}

#[allow(dead_code)]
pub fn enabled_categories(toggles: &ArtifactToggles) -> Vec<&'static CategoryDescriptor> {
    REGISTRY
        .iter()
        .filter(|d| toggles.is_enabled(d.id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_registry_has_all_twelve_categories() {
        assert_eq!(REGISTRY.len(), 12);
        let ids: HashSet<_> = REGISTRY.iter().map(|d| d.id).collect();
        assert_eq!(ids.len(), 12, "every category appears exactly once");
    }

    #[test]
    fn test_registry_names_and_subdirs_unique() {
        let names: HashSet<_> = REGISTRY.iter().map(|d| d.name).collect();
        let subdirs: HashSet<_> = REGISTRY.iter().map(|d| d.repo_subdir).collect();
        assert_eq!(names.len(), REGISTRY.len());
        assert_eq!(subdirs.len(), REGISTRY.len());
    }

    #[test]
    fn test_plugins_category_is_exact_file_allowlist() {
        let plugins = REGISTRY
            .iter()
            .find(|d| d.id == CategoryId::Plugins)
            .unwrap();
        match plugins.source {
            SourceSpec::Files(files) => {
                assert_eq!(
                    files,
                    [
                        "plugins/installed_plugins.json",
                        "plugins/known_marketplaces.json"
                    ]
                );
            }
            _ => panic!("plugins must be an exact file allowlist, never a dir walk"),
        }
    }

    #[test]
    fn test_prompt_history_uses_union_merge() {
        let ph = REGISTRY
            .iter()
            .find(|d| d.id == CategoryId::PromptHistory)
            .unwrap();
        assert_eq!(ph.merge, MergeStrategy::UnionJsonl);
        assert_eq!(ph.source, SourceSpec::Files(&["history.jsonl"]));
        for d in REGISTRY
            .iter()
            .filter(|d| d.id != CategoryId::PromptHistory && d.id != CategoryId::ProjectAttachments)
        {
            assert_eq!(d.merge, MergeStrategy::RawOverwrite, "{}", d.name);
        }
        let attachments = REGISTRY
            .iter()
            .find(|d| d.id == CategoryId::ProjectAttachments)
            .unwrap();
        assert_eq!(attachments.merge, MergeStrategy::UnionMemoryIndex);
    }

    #[test]
    fn test_only_curated_directories_mirror_deletions() {
        let mirrored: HashSet<_> = REGISTRY
            .iter()
            .filter(|d| d.mirror_deletes)
            .map(|d| d.name)
            .collect();
        assert_eq!(
            mirrored,
            HashSet::from(["skills", "agents", "commands", "rules", "hooks"])
        );
        for d in REGISTRY.iter().filter(|d| d.mirror_deletes) {
            assert!(
                matches!(d.source, SourceSpec::Dir(_)),
                "{} mirrors deletions, so its source must be a directory",
                d.name
            );
            assert_eq!(
                d.dest,
                DestRoot::Artifacts,
                "{} mirrors deletions, so every tracked path must resolve to a \
                 destination — a session-tree category can have none",
                d.name
            );
        }
    }

    #[test]
    fn test_only_config_files_are_path_tokenized() {
        let tokenized: HashSet<_> = REGISTRY
            .iter()
            .filter(|d| d.tokenize_paths)
            .map(|d| d.name)
            .collect();
        assert_eq!(tokenized, HashSet::from(["settings", "plugins"]));
    }

    #[test]
    fn test_settings_never_includes_local_overrides() {
        let settings = REGISTRY
            .iter()
            .find(|d| d.id == CategoryId::Settings)
            .unwrap();
        match settings.source {
            SourceSpec::Files(files) => {
                assert!(files.contains(&"settings.json"));
                assert!(files.contains(&"keybindings.json"));
                assert!(!files.contains(&"settings.local.json"));
            }
            _ => panic!("settings must be an exact file allowlist"),
        }
    }

    #[test]
    fn test_toggles_map_every_category() {
        let mut toggles = ArtifactToggles::default();
        assert!(!toggles.any_enabled());
        for d in toggleable() {
            assert!(!toggles.is_enabled(d.id), "{} defaults off", d.name);
            toggles.set_enabled(d.id, true);
            assert!(toggles.is_enabled(d.id), "{} can be enabled", d.name);
        }
        assert_eq!(toggles, ArtifactToggles::all_enabled());
        assert!(!toggles.is_enabled(CategoryId::ProjectAttachments));
        toggles.set_enabled(CategoryId::ProjectAttachments, true);
        assert!(!toggles.is_enabled(CategoryId::ProjectAttachments));
    }

    #[test]
    fn test_find_by_name_matches_cli_names() {
        for d in REGISTRY {
            let found = find_by_name(d.name).unwrap();
            assert_eq!(found.id, d.id);
        }
        assert!(find_by_name("prompt-history").is_some());
        assert!(find_by_name("nonsense").is_none());
    }

    #[test]
    fn test_old_config_without_toggles_parses_all_off() {
        let toggles: ArtifactToggles = toml::from_str("").unwrap();
        assert_eq!(toggles, ArtifactToggles::default());
        assert!(!toggles.any_enabled());
    }
}
