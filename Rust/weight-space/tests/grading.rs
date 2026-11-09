use claw_weight_space::evaluation::{Case, Expected, grade, repeated_ngram, synthetic_cases};

fn math() -> Case {
    Case {
        id: "test".into(),
        prompt: "integer".into(),
        expected: Expected::Integer(42),
    }
}
#[test]
fn final_answers_only() {
    assert!(grade(
        &math(),
        "42 in thought</think>Answer: \\boxed{42}",
        true
    ));
    assert!(!grade(&math(), "\\boxed{42} but still thinking", true));
    assert!(!grade(&math(), "\\boxed{42}</think>\\boxed{41}", true));
    assert!(!grade(&math(), "</think>\\boxed{42.0}", true));
    assert!(!grade(&math(), "</think>\\boxed{42", true));
    assert!(grade(&math(), "\\boxed{42}", false));
    assert!(!grade(
        &math(),
        "</think>\\boxed{42} then \\boxed{43}",
        true
    ));
}
#[test]
fn reproducible_disjoint_cases() {
    let a = synthetic_cases(17, 24).unwrap();
    let b = synthetic_cases(18, 24).unwrap();
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&synthetic_cases(17, 24).unwrap()).unwrap()
    );
    assert!(!a.iter().any(|x| b.iter().any(|y| x.prompt == y.prompt)));
    assert!(synthetic_cases(1, 0).is_err());
}
#[test]
fn loops_require_repeated_token_sequences() {
    assert!(repeated_ngram(
        &(0..32).cycle().take(256).collect::<Vec<_>>()
    ));
    assert!(!repeated_ngram(&(0..256).collect::<Vec<_>>()));
    assert!(!repeated_ngram(&[1; 32]));
}
