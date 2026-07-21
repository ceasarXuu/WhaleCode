use super::*;

#[test]
fn source_inventory_tracks_every_rust_file_change() {
    let temp = tempfile::tempdir().expect("temporary repository");
    for (index, root) in SOURCE_ROOTS.iter().enumerate() {
        let directory = temp.path().join(root);
        fs::create_dir_all(&directory).expect("source root");
        fs::write(
            directory.join(format!("root_{index}.rs")),
            "pub fn root() {}\n",
        )
        .expect("source fixture");
    }

    let initial = index_sources(temp.path()).expect("initial inventory");
    let initial_sources = initial.all_source_hashes();
    assert_eq!(initial_sources.len(), SOURCE_ROOTS.len());
    let initial_hash = canonical_hash(&initial_sources).expect("initial hash");

    let modified_path = temp.path().join(SOURCE_ROOTS[0]).join("root_0.rs");
    fs::write(&modified_path, "pub fn changed() {}\n").expect("modify source");
    let modified_sources = index_sources(temp.path())
        .expect("modified inventory")
        .all_source_hashes();
    assert_ne!(
        canonical_hash(&modified_sources).expect("modified hash"),
        initial_hash
    );

    let added_path = temp.path().join(SOURCE_ROOTS[1]).join("added.rs");
    fs::write(&added_path, "pub struct Added;\n").expect("add source");
    let added_sources = index_sources(temp.path())
        .expect("added inventory")
        .all_source_hashes();
    assert_eq!(added_sources.len(), SOURCE_ROOTS.len() + 1);
    assert_ne!(
        canonical_hash(&added_sources).expect("added hash"),
        canonical_hash(&modified_sources).expect("modified hash")
    );

    fs::remove_file(added_path).expect("remove source");
    let removed_sources = index_sources(temp.path())
        .expect("removed inventory")
        .all_source_hashes();
    assert_eq!(removed_sources, modified_sources);
}
