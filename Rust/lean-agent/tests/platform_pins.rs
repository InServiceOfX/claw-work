use claw_lean_agent::proof::validate_image_pins;

#[test]
fn only_exact_known_architecture_profiles_are_accepted() {
    for profile in [
        include_str!("../docker/pins.json"),
        include_str!("../docker/pins-arm64.json"),
    ] {
        let pins: serde_json::Value = serde_json::from_str(profile).unwrap();
        validate_image_pins(&pins).unwrap();
        for field in ["lean_archive_sha256", "mathlib_commit", "platform"] {
            let mut changed = pins.clone();
            changed[field] = "untrusted".into();
            assert!(validate_image_pins(&changed).is_err(), "{field}");
        }
        let mut extra = pins.clone();
        extra["unreviewed"] = true.into();
        assert!(validate_image_pins(&extra).is_err());
    }
}
