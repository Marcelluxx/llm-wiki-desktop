use std::fs;

use llm_wiki_app_core::{
    ProviderId, RegistryError, RegistryStore, ensure_obsidian_visibility, ensure_wiki_agents_file,
    remove_legacy_source_properties,
};
use tempfile::tempdir;

#[test]
fn creates_two_isolated_wikis_and_removes_only_the_registration() {
    let sandbox = tempdir().expect("temporary sandbox");
    let app_data = sandbox.path().join("app-data");
    let wiki_parent = sandbox.path().join("wikis");
    fs::create_dir_all(&wiki_parent).expect("wiki parent");
    let store = RegistryStore::new(&app_data, None, None);

    let first = store
        .create_wiki("Prima wiki", wiki_parent.join("prima"), "it")
        .expect("first wiki");
    let second = store
        .create_wiki("Second wiki", wiki_parent.join("seconda"), "en")
        .expect("second wiki");

    assert_ne!(first.wiki_id, second.wiki_id);
    assert!(wiki_parent.join("prima/.llm-wiki/settings.json").is_file());
    assert!(wiki_parent.join("seconda/concepts").is_dir());
    assert_eq!(store.snapshot().expect("snapshot").wikis.len(), 2);

    store
        .remove_registration(&first.wiki_id)
        .expect("remove registration");

    assert_eq!(store.snapshot().expect("snapshot").wikis.len(), 1);
    assert!(wiki_parent.join("prima/index.md").is_file());
    assert!(wiki_parent.join("prima/AGENTS.md").is_file());
    assert!(wiki_parent.join("prima/.llm-wiki").is_dir());
}

#[test]
fn rejects_forbidden_duplicate_nested_and_missing_paths() {
    let sandbox = tempdir().expect("temporary sandbox");
    let app_data = sandbox.path().join("app-data");
    let wiki_parent = sandbox.path().join("wikis");
    fs::create_dir_all(&wiki_parent).expect("wiki parent");
    let store = RegistryStore::new(&app_data, Some(wiki_parent.clone()), None);

    assert!(matches!(
        store.create_wiki("Too broad", &wiki_parent, "it"),
        Err(RegistryError::ForbiddenRoot)
    ));

    let allowed_parent = sandbox.path().join("allowed");
    fs::create_dir_all(&allowed_parent).expect("allowed parent");
    let root = allowed_parent.join("wiki");
    store.create_wiki("Valid", &root, "it").expect("valid wiki");

    assert!(matches!(
        store.create_wiki("Duplicate", &root, "it"),
        Err(RegistryError::DuplicateOrNestedPath)
    ));
    assert!(matches!(
        store.create_wiki("Nested", root.join("nested"), "it"),
        Err(RegistryError::DuplicateOrNestedPath)
    ));
    assert!(matches!(
        store.register_wiki("Missing", allowed_parent.join("missing"), "it"),
        Err(RegistryError::MissingPath)
    ));
}

#[test]
fn preserves_a_stable_id_when_an_existing_wiki_is_registered_again() {
    let sandbox = tempdir().expect("temporary sandbox");
    let root = sandbox.path().join("wiki");
    fs::create_dir_all(sandbox.path().join("first-app-data")).expect("first app data");
    let first_store = RegistryStore::new(sandbox.path().join("first-app-data"), None, None);
    let first = first_store
        .create_wiki("Wiki", &root, "it")
        .expect("create wiki");
    fs::write(root.join("AGENTS.md"), "# Regole personalizzate\n").expect("custom agents");

    let second_store = RegistryStore::new(sandbox.path().join("second-app-data"), None, None);
    let registered = second_store
        .register_wiki("Wiki recuperata", &root, "it")
        .expect("register existing wiki");

    assert_eq!(registered.wiki_id, first.wiki_id);
    assert_eq!(
        fs::read_to_string(root.join("AGENTS.md")).expect("read agents"),
        "# Regole personalizzate\n"
    );
}

#[test]
fn persists_one_active_provider() {
    let sandbox = tempdir().expect("temporary sandbox");
    let store = RegistryStore::new(sandbox.path().join("app-data"), None, None);

    store
        .set_selected_provider(ProviderId::Claude)
        .expect("select Claude");
    assert_eq!(
        store.snapshot().expect("snapshot").selected_provider_id,
        Some(ProviderId::Claude)
    );

    store
        .set_selected_provider(ProviderId::Antigravity)
        .expect("select Antigravity");
    assert_eq!(
        store.snapshot().expect("snapshot").selected_provider_id,
        Some(ProviderId::Antigravity)
    );
}

#[test]
fn preserves_obsidian_settings_while_hiding_internal_ingest_files() {
    let sandbox = tempdir().expect("temporary sandbox");
    let root = sandbox.path().join("wiki");
    let obsidian = root.join(".obsidian");
    fs::create_dir_all(&obsidian).expect("obsidian folder");
    fs::write(
        obsidian.join("app.json"),
        r#"{"userIgnoreFilters":["private/"],"showUnsupportedFiles":true}"#,
    )
    .expect("app settings");
    fs::write(
        obsidian.join("graph.json"),
        r#"{"search":"tag:#research","showAttachments":true,"showOrphans":false}"#,
    )
    .expect("graph settings");

    ensure_obsidian_visibility(&root).expect("configure visibility");

    let app: serde_json::Value =
        serde_json::from_slice(&fs::read(obsidian.join("app.json")).expect("read app settings"))
            .expect("valid app settings");
    assert_eq!(app["showUnsupportedFiles"], true);
    assert_eq!(
        app["userIgnoreFilters"],
        serde_json::json!(["private/", ".llm-wiki/"])
    );
    let graph: serde_json::Value = serde_json::from_slice(
        &fs::read(obsidian.join("graph.json")).expect("read graph settings"),
    )
    .expect("valid graph settings");
    assert_eq!(graph["showAttachments"], false);
    assert_eq!(graph["showOrphans"], false);
    assert_eq!(graph["search"], "tag:#research -path:\".llm-wiki\"");
}

#[test]
fn migrates_only_the_managed_legacy_agents_schema() {
    let sandbox = tempdir().expect("temporary sandbox");
    let root = sandbox.path().join("wiki");
    fs::create_dir_all(&root).expect("wiki root");
    fs::write(
        root.join("AGENTS.md"),
        "# LLM Wiki knowledge-ingest blueprint\nsource_ids: [\"sha256 when evidence-backed\"]\n- sources processed and cache identities used;\n",
    )
    .expect("legacy agents");

    ensure_wiki_agents_file(&root).expect("migrate agents");

    let migrated = fs::read_to_string(root.join("AGENTS.md")).expect("read migrated agents");
    assert!(migrated.starts_with("<!-- llm-wiki-agents-version: 2 -->"));
    assert!(!migrated.contains("source_ids: ["));
    assert!(migrated.contains("- sources processed;"));
}

#[test]
fn removes_legacy_source_ids_only_from_managed_markdown_frontmatter() {
    let sandbox = tempdir().expect("temporary sandbox");
    let root = sandbox.path().join("wiki");
    fs::create_dir_all(root.join("entities")).expect("entities directory");
    fs::create_dir_all(root.join(".llm-wiki/artifacts")).expect("internal directory");
    let note = root.join("entities/example.md");
    fs::write(
        &note,
        "---\ntitle: Example\nsource_id: abc\nsource_ids:\n  - def\n  - ghi\ntags: [test]\n---\n# Example\n\nsource_ids: keep this body text\n",
    )
    .expect("managed note");
    let internal = root.join(".llm-wiki/artifacts/document.md");
    fs::write(&internal, "---\nsource_ids: [internal]\n---\n").expect("internal note");

    let updated = remove_legacy_source_properties(&root).expect("migration succeeds");

    assert_eq!(updated, 1);
    let migrated = fs::read_to_string(note).expect("migrated note");
    assert!(!migrated.contains("source_id: abc"));
    assert!(!migrated.contains("  - def"));
    assert!(migrated.contains("tags: [test]"));
    assert!(migrated.contains("source_ids: keep this body text"));
    assert!(
        fs::read_to_string(internal)
            .expect("internal note remains")
            .contains("source_ids: [internal]")
    );
}
