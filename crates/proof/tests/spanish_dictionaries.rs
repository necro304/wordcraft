//! Exercise the public dictionary-folder API in a separate process: the library's unit tests
//! install their own dictionaries, so changing the Spanish cache here cannot race with them.

use wordcraft_proof::{self as proof, IssueKind, ProofLang};

#[test]
fn spanish_dictionary_folder_reloads_and_reports_its_source() {
    let root = std::env::temp_dir().join(format!("wordcraft-spanish-dictionaries-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let first = root.join("first");
    let second = root.join("second");
    let invalid = root.join("invalid");
    for dir in [&first, &second, &invalid] {
        std::fs::create_dir(dir).unwrap();
    }
    // Original, minimal fixtures, not copied from an external dictionary.
    let aff = "SET UTF-8\nMAP 3\nMAP aá\nMAP oó\nMAP nñ\n";
    std::fs::write(first.join("es_MX.aff"), aff).unwrap();
    std::fs::write(first.join("es_MX.dic"), "2\ncanción\nniño\n").unwrap();
    std::fs::write(second.join("es_CO.aff"), aff).unwrap();
    std::fs::write(second.join("es_CO.dic"), "2\nárbol\nmañana\n").unwrap();
    std::fs::write(invalid.join("es_419.aff"), "FLAG long\n").unwrap();
    std::fs::write(invalid.join("es_419.dic"), "1\ncanción\n").unwrap();

    proof::set_dictionaries_folder(Some(first.clone()));
    assert!(proof::is_correct_for("canción", Some("es-419")));
    assert!(!proof::is_correct_for("cancion", Some("es-MX")));
    assert_eq!(proof::suggest_in("nino", 3, ProofLang::Es).first().map(String::as_str), Some("niño"));
    assert_eq!(proof::missing_dictionary(Some("es-419")), None);
    let generation = proof::dictionary_generation();

    proof::set_dictionaries_folder(Some(second.clone()));
    assert!(proof::dictionary_generation() > generation);
    assert!(proof::is_correct_for("árbol", Some("es-CO")), "changing the folder must reload Spanish, not retain the first dictionary");
    assert!(!proof::is_correct_for("canción", Some("es-419")), "words from the old dictionary must no longer be accepted");
    for (dir, name) in [(&first, "es_MX"), (&second, "es_CO")] {
        proof::set_dictionaries_folder(Some(dir.clone()));
        assert_eq!(
            proof::installed_dictionaries().into_iter().find(|(lang, _)| *lang == ProofLang::Es),
            Some((ProofLang::Es, Some(dir.join(format!("{name}.dic"))))),
            "the Proofing options must report the Spanish dictionary actually selected"
        );
    }

    // A dictionary that cannot be read has the same visible hint as a missing one. It must
    // also clear the old dictionary rather than marking Spanish words against stale data.
    proof::set_dictionaries_folder(Some(invalid));
    for tag in ["es", "es-419", "es-MX", "es-ES", "spa"] {
        assert_eq!(proof::missing_dictionary(Some(tag)), Some("es_419"), "{tag}");
        assert!(proof::is_correct_for("cancion", Some(tag)));
    }
    let text = "¿hola?";
    let issues = proof::check_text(text, &[(0..text.len(), ProofLang::Es)]);
    assert_eq!(issues.len(), 1, "grammar remains available without a spelling dictionary: {issues:?}");
    assert_eq!(issues[0].kind, IssueKind::Grammar);
    assert_eq!(issues[0].suggestions, ["Hola"]);

    proof::set_dictionaries_folder(None);
    std::fs::remove_dir_all(root).unwrap();
}
