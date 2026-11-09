use claw_weight_space::projection::project;

#[test]
fn projection_obeys_geometry_and_selection() {
    let w = vec![3., 4., 5., 6., 7., 8.]; // [2,3]
    let p = project(&w, 2, 3, &[1., 2.], &[0, 2], 1.).unwrap();
    for c in [0, 2] {
        assert!((p[c] + 2. * p[3 + c]).abs() < 1e-5);
    }
    assert_eq!(p[1], w[1]);
    assert_eq!(p[4], w[4]);
    assert_eq!(project(&w, 2, 3, &[1., 2.], &[0], 0.).unwrap(), w);
    assert_eq!(w, vec![3., 4., 5., 6., 7., 8.]);
}

#[test]
fn invalid_projection_is_rejected() {
    for (u, cols, a) in [
        (vec![0., 0.], vec![0], 1.),
        (vec![f32::NAN, 1.], vec![0], 1.),
        (vec![1., 1.], vec![0, 0], 1.),
        (vec![1., 1.], vec![2], 1.),
        (vec![1., 1.], vec![0], f32::INFINITY),
    ] {
        assert!(project(&[1., 2., 3., 4.], 2, 2, &u, &cols, a).is_err());
    }
    assert!(project(&[1., 2.], 2, 2, &[1., 1.], &[0], 1.).is_err());
}
