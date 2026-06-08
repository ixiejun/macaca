    use super::*;

    #[test]
    fn default_clients_exist() {
        // This test only works if HOME is set.
        if std::env::var("HOME").is_ok() {
            let clients = default_clients();
            assert!(clients.len() >= 4);

            let names: Vec<&str> = clients.iter().map(|c| c.name.as_str()).collect();
            assert!(names.contains(&ANTHROPIC_DESKTOP_CLIENT_ID));
            assert!(names.contains(&"cursor"));
        }
    }

    #[tokio::test]
    async fn provision_all_for_client() {
        let central = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();

        // Create skills in central store.
        for name in &["golang", "shadcn-ui"] {
            let skill_dir = central.path().join(name);
            tokio::fs::create_dir(&skill_dir).await.unwrap();
            tokio::fs::write(
                skill_dir.join("SKILL.md"),
                format!("---\nname: {name}\ndescription: {name} skill\n---\n# {name}\nContent."),
            )
            .await
            .unwrap();
        }

        let mut provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());
        provisioner.register_client(ClientConfig {
            name: "test-client".into(),
            skills_dir: target.path().join("skills"),
        });

        let count = provisioner
            .provision_all_for_client("test-client")
            .await
            .unwrap();
        assert_eq!(count, 2);

        // Verify files were copied.
        assert!(target.path().join("skills/golang/SKILL.md").exists());
        assert!(target.path().join("skills/shadcn-ui/SKILL.md").exists());
    }

    #[tokio::test]
    async fn provision_specific_skill() {
        let central = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();

        // Create one skill.
        let skill_dir = central.path().join("golang");
        tokio::fs::create_dir(&skill_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: golang\ndescription: Go\n---\n# Go\nContent.",
        )
        .await
        .unwrap();
        // Add a bundled resource.
        tokio::fs::write(skill_dir.join("helpers.sh"), "#!/bin/bash\necho hi")
            .await
            .unwrap();

        let mut provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());
        provisioner.register_client(ClientConfig {
            name: "test-client".into(),
            skills_dir: target.path().join("skills"),
        });

        provisioner
            .provision_skill("golang", "test-client")
            .await
            .unwrap();

        // Both SKILL.md and helpers.sh should be copied.
        assert!(target.path().join("skills/golang/SKILL.md").exists());
        assert!(target.path().join("skills/golang/helpers.sh").exists());
    }

    #[tokio::test]
    async fn provision_skill_with_handle_returns_state() {
        let central = tempfile::tempdir().unwrap();
        let client = tempfile::tempdir().unwrap();
        let skill_dir = central.path().join("browser");
        tokio::fs::create_dir_all(&skill_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: browser\ndescription: Browser\n---\nBody",
        )
        .await
        .unwrap();

        let mut provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());
        provisioner.register_client(ClientConfig {
            name: "test-client".into(),
            skills_dir: client.path().to_path_buf(),
        });

        let handle = provisioner
            .provision_skill_with_handle("browser", "test-client")
            .await
            .unwrap();

        assert_eq!(handle.skill_id, "browser");
        assert_eq!(handle.client_id, "test-client");
        assert_eq!(handle.state, crate::handle::SkillRuntimeState::Provisioned);
        assert!(handle.target_dir.join("SKILL.md").exists());
    }

    #[tokio::test]
    async fn provision_missing_skill() {
        let central = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();

        let mut provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());
        provisioner.register_client(ClientConfig {
            name: "test-client".into(),
            skills_dir: target.path().join("skills"),
        });

        let err = provisioner
            .provision_skill("nonexistent", "test-client")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn provision_unknown_client() {
        let central = tempfile::tempdir().unwrap();
        let provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());

        let err = provisioner
            .provision_skill("golang", "unknown-client")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Unknown client"));
    }

    #[tokio::test]
    async fn list_central_skills() {
        let central = tempfile::tempdir().unwrap();

        for name in &["alpha", "beta"] {
            let skill_dir = central.path().join(name);
            tokio::fs::create_dir(&skill_dir).await.unwrap();
            tokio::fs::write(
                skill_dir.join("SKILL.md"),
                format!("---\nname: {name}\ndescription: test\n---\nbody"),
            )
            .await
            .unwrap();
        }

        // Also create a non-skill directory.
        let not_skill = central.path().join("not-a-skill");
        tokio::fs::create_dir(&not_skill).await.unwrap();
        tokio::fs::write(not_skill.join("README.md"), "not a skill")
            .await
            .unwrap();

        let provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());
        let skills = provisioner.list_central_skills().await.unwrap();
        assert_eq!(skills, vec!["alpha", "beta"]);
    }

    #[tokio::test]
    async fn provision_with_subdirectories() {
        let central = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();

        // Create a skill with a scripts/ subdirectory.
        let skill_dir = central.path().join("my-skill");
        let scripts_dir = skill_dir.join("scripts");
        tokio::fs::create_dir_all(&scripts_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: test\n---\nbody",
        )
        .await
        .unwrap();
        tokio::fs::write(scripts_dir.join("helper.py"), "print('hello')")
            .await
            .unwrap();

        let mut provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());
        provisioner.register_client(ClientConfig {
            name: "test-client".into(),
            skills_dir: target.path().join("skills"),
        });

        provisioner
            .provision_skill("my-skill", "test-client")
            .await
            .unwrap();

        // Verify recursive copy.
        assert!(target.path().join("skills/my-skill/SKILL.md").exists());
        assert!(target
            .path()
            .join("skills/my-skill/scripts/helper.py")
            .exists());
    }
