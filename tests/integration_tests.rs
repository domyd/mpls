use mpls::Mpls;

#[test]
fn complete_tiny() {
    // tiny mpls, not a main feature (00770), only 214 bytes, single segment
    let data = include_bytes!("../assets/tiny.mpls");
    let sl = &data[..];

    let res = Mpls::from(sl);
    assert!(res.is_ok());
}

#[test]
fn complete_small() {
    // simpler mpls, main feature with only a few segments
    let data = include_bytes!("../assets/simple.mpls");
    let sl = &data[..];

    let res = Mpls::from(sl);
    assert!(res.is_ok());
}

#[test]
fn complete_large() {
    // more complex mpls, main feature with many many segments
    let data = include_bytes!("../assets/large.mpls");
    let sl = &data[..];

    let res = Mpls::from(sl);
    assert!(res.is_ok());
}

#[test]
fn complete_multi_angle() {
    // large mpls with multiple angles
    let data = include_bytes!("../assets/multi-angle.mpls");
    let sl = &data[..];

    let res = Mpls::from(sl);
    assert!(res.is_ok());
}

#[test]
fn multi_angle_count() {
    let data = include_bytes!("../assets/multi-angle.mpls");
    let sl = &data[..];

    let mpls = Mpls::from(sl).unwrap();
    assert_eq!(mpls.angles().len(), 4);
}

#[test]
fn single_angle_count() {
    let data = include_bytes!("../assets/simple.mpls");
    let sl = &data[..];

    let mpls = Mpls::from(sl).unwrap();
    assert_eq!(mpls.angles().len(), 1);
}

#[test]
fn single_angle_segments() {
    let data = include_bytes!("../assets/simple.mpls");
    let sl = &data[..];

    let mpls = Mpls::from(sl).unwrap();
    let angle = mpls.angles()[0];
    let segments: Vec<&str> = angle
        .segments()
        .iter()
        .map(|s| s.file_name.as_ref())
        .collect();

    assert_eq!(segments, &["00055", "00059", "00061"]);
}

#[test]
fn multi_angle_first_segments() {
    let data = include_bytes!("../assets/multi-angle.mpls");
    let sl = &data[..];

    let mpls = Mpls::from(sl).unwrap();
    let angle = mpls.angles()[0];
    let segments: Vec<&str> = angle
        .segments()
        .iter()
        .map(|s| s.file_name.as_ref())
        .collect();

    assert_eq!(
        &segments[..5],
        &["00081", "00082", "00086", "00087", "00091"]
    );
}

#[test]
fn multi_angle_last_segments() {
    let data = include_bytes!("../assets/multi-angle.mpls");
    let sl = &data[..];

    let mpls = Mpls::from(sl).unwrap();
    let angle = mpls.angles()[3];
    let segments: Vec<&str> = angle
        .segments()
        .iter()
        .map(|s| s.file_name.as_ref())
        .collect();

    assert_eq!(
        &segments[..5],
        &["00081", "00085", "00086", "00090", "00091"]
    );
}
