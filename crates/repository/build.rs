fn main() {
    println!("cargo:rerun-if-env-changed=VERZLY_SCHEMA_REF");

    let schema_ref = std::env::var("VERZLY_SCHEMA_REF")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "local".to_string());

    println!("cargo:rustc-env=VERZLY_SCHEMA_REF={schema_ref}");
}
