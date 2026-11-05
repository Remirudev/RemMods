use proptest::prelude::*;
use proptest::test_runner::Config;
use remmods_pak::fixture::TocBuilder;
use remmods_pak::read_toc_bytes;

const MOUNT: &str = "../../../Polaris/Content/";

fn segment() -> impl Strategy<Value = String> {
    "[A-Za-z0-9_ äÿ.-]{1,8}".prop_filter("not a dot name", |s| s != "." && s != "..")
}

fn tree() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(prop::collection::vec(segment(), 1..5), 0..24)
        .prop_map(|paths| paths.into_iter().map(|p| p.join("/")).collect())
}

fn sample() -> Vec<u8> {
    TocBuilder::new(MOUNT)
        .file("Smoke/A.uasset")
        .file("Smoke/b/B.ubulk")
        .file("C.uasset")
        .signed(8)
        .build()
}

proptest! {
    #![proptest_config(Config { failure_persistence: None, ..Config::default() })]

    #[test]
    fn random_trees_round_trip(paths in tree(), utf16 in any::<bool>()) {
        let mut builder = TocBuilder::new(MOUNT).utf16_strings(utf16);
        for path in &paths {
            builder = builder.file(path);
        }
        let listing = read_toc_bytes(&builder.build()).unwrap();
        let mut expected: Vec<String> =
            paths.iter().map(|p| format!("Polaris/Content/{p}")).collect();
        expected.sort();
        let got: Vec<String> = listing.files.into_iter().map(|f| f.path).collect();
        prop_assert_eq!(got, expected);
    }

    #[test]
    fn corrupted_bytes_never_panic(at in any::<prop::sample::Index>(), value in any::<u8>(), cut in any::<prop::sample::Index>()) {
        let mut bytes = sample();
        let i = at.index(bytes.len());
        bytes[i] = value;
        let _ = read_toc_bytes(&bytes);
        let len = cut.index(bytes.len() + 1);
        let _ = read_toc_bytes(&bytes[..len]);
    }

    #[test]
    fn random_bytes_never_panic(tail in prop::collection::vec(any::<u8>(), 0..2048), version in 0u8..10) {
        let _ = read_toc_bytes(&tail);
        let mut bytes = sample()[..144].to_vec();
        bytes[16] = version;
        bytes.extend_from_slice(&tail);
        let _ = read_toc_bytes(&bytes);
        let _ = remmods_pak::parse_directory_index(&tail);
    }
}
