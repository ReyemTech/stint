fn main() {
    // option_env! reads these at compile time. Without these directives,
    // cargo doesn't know to rebuild when the values change.
    println!("cargo:rerun-if-env-changed=STINT_GOOGLE_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=STINT_GOOGLE_CLIENT_SECRET");
}
