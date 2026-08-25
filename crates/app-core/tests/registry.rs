use std::fs;

use llm_wiki_app_core::{RegistryError, RegistryStore};
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

    let second_store = RegistryStore::new(sandbox.path().join("second-app-data"), None, None);
    let registered = second_store
        .register_wiki("Wiki recuperata", &root, "it")
        .expect("register existing wiki");

    assert_eq!(registered.wiki_id, first.wiki_id);
}
